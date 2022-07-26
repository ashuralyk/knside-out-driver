use std::time::Duration;

use ckb_hash::new_blake2b;
use ko_protocol::ckb_jsonrpc_types::{OutputsValidator, Status, TransactionView as JsonTxView};
use ko_protocol::ckb_sdk::SECP256K1;
use ko_protocol::ckb_types::packed::WitnessArgs;
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack};
use ko_protocol::ckb_types::{bytes::Bytes, core::TransactionView};
use ko_protocol::secp256k1::{Message, SecretKey};
use ko_protocol::serde_json::to_string;
use ko_protocol::traits::{CkbClient, Driver};
use ko_protocol::{async_trait, log, tokio, KoResult, H256};

mod error;
use error::DriverError;

pub struct DriverImpl<C: CkbClient> {
    rpc_client: C,
    privkey: SecretKey,
}

impl<C: CkbClient> DriverImpl<C> {
    pub fn new(rpc_client: &C, privkey: &SecretKey) -> DriverImpl<C> {
        DriverImpl {
            rpc_client: rpc_client.clone(),
            privkey: *privkey,
        }
    }
}

#[async_trait]
impl<C: CkbClient> Driver for DriverImpl<C> {
    fn sign_transaction(&self, tx: &TransactionView) -> Bytes {
        let mut blake2b = new_blake2b();
        blake2b.update(&tx.hash().raw_data());
        // prepare empty witness for digest
        let witness_for_digest = WitnessArgs::new_builder()
            .lock(Some(Bytes::from(vec![0u8; 65])).pack())
            .build();
        // hash witness message
        let mut message = [0u8; 32];
        let witness_len = witness_for_digest.as_bytes().len() as u64;
        blake2b.update(&witness_len.to_le_bytes());
        blake2b.update(&witness_for_digest.as_bytes());
        blake2b.finalize(&mut message);
        let digest = Message::from_slice(&message).unwrap();
        // sign digest message
        let signature = SECP256K1.sign_recoverable(&digest, &self.privkey);
        let signature_bytes = {
            let (recover_id, signature) = signature.serialize_compact();
            let mut bytes = signature.to_vec();
            bytes.push(recover_id.to_i32() as u8);
            bytes
        };
        Bytes::from(signature_bytes)
    }

    async fn send_transaction(&self, tx: TransactionView) -> KoResult<H256> {
        let hash = self
            .rpc_client
            .send_transaction(&tx.data().into(), Some(OutputsValidator::Passthrough))
            .await
            .map_err(|err| {
                DriverError::TransactionSendError(
                    err.to_string(),
                    to_string(&JsonTxView::from(tx)).unwrap(),
                )
            })?;
        Ok(hash)
    }

    async fn wait_transaction_committed(
        &mut self,
        hash: &H256,
        interval: &Duration,
        confirms: u8,
    ) -> KoResult<()> {
        let mut block_number = 0u64;
        loop {
            tokio::time::sleep(*interval).await;
            let tx = self
                .rpc_client
                .get_transaction(hash)
                .await
                .map_err(|err| DriverError::TransactionFetchError(err.to_string(), hash.clone()))?
                .unwrap();
            if tx.tx_status.status == Status::Rejected {
                return Err(DriverError::TransactionFetchError(
                    tx.tx_status.reason.unwrap_or_else(|| "rejected".into()),
                    hash.clone(),
                )
                .into());
            }
            if tx.tx_status.status != Status::Committed {
                continue;
            }
            if block_number == 0 {
                if let Some(block_hash) = tx.tx_status.block_hash {
                    let block = self.rpc_client.get_block(&block_hash.into()).await.unwrap();
                    block_number = block.header.inner.number.into();
                    log::info!(
                        "transaction #{} commited in block #{}, wait confirm...",
                        hash,
                        block_number
                    );
                }
            } else {
                let tip = self.rpc_client.get_tip_header().await.unwrap();
                let tip_number: u64 = tip.inner.number.into();
                if tip_number >= block_number + confirms as u64 {
                    break;
                }
            }
        }
        Ok(())
    }
}
