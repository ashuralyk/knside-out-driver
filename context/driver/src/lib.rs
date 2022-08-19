use std::collections::HashMap;
use std::time::Duration;

use ckb_hash::new_blake2b;
use ko_protocol::ckb_jsonrpc_types::{OutputsValidator, Status, TransactionView as JsonTxView};
use ko_protocol::ckb_sdk::SECP256K1;
use ko_protocol::ckb_types::packed::{Transaction, WitnessArgs};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack, Unpack};
use ko_protocol::ckb_types::{bytes::Bytes, core::TransactionView, H256};
use ko_protocol::secp256k1::{Message, SecretKey};
use ko_protocol::serde_json::to_string;
use ko_protocol::tokio::sync::mpsc::UnboundedSender;
use ko_protocol::traits::{CkbClient, Driver};
use ko_protocol::{async_trait, tokio, KoResult};

mod error;
use error::DriverError;

pub struct DriverImpl<C: CkbClient> {
    rpc_client: C,
    privkey: SecretKey,
    listening_requests: HashMap<H256, UnboundedSender<H256>>,
}

impl<C: CkbClient> DriverImpl<C> {
    pub fn new(rpc_client: &C, privkey: &SecretKey) -> DriverImpl<C> {
        DriverImpl {
            rpc_client: rpc_client.clone(),
            privkey: *privkey,
            listening_requests: HashMap::new(),
        }
    }

    pub fn add_callback_request_hash(
        &mut self,
        request_hash: &H256,
        sender: UnboundedSender<H256>,
    ) {
        self.listening_requests.insert(request_hash.clone(), sender);
    }
}

#[async_trait]
impl<C: CkbClient> Driver for DriverImpl<C> {
    fn sign_ko_transaction(&self, tx: &TransactionView) -> Bytes {
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

    async fn send_ko_transaction(&self, tx: TransactionView) -> KoResult<H256> {
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

    async fn wait_ko_transaction_committed(
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
                    let block = self.rpc_client.get_block(&block_hash).await.unwrap();
                    block_number = block.header.inner.number.into();
                    println!(
                        "[INFO] transaction commited in block #{}, wait confirm...",
                        block_number
                    );
                }
            } else {
                let tip = self.rpc_client.get_tip_header().await.unwrap();
                let tip_number: u64 = tip.inner.number.into();
                if tip_number > block_number + confirms as u64 {
                    println!("[INFO] transaction confirmed");
                    // clear request listening callbacks
                    let out_points = Transaction::from(tx.transaction.unwrap().inner)
                        .into_view()
                        .inputs();
                    for i in 0..out_points.len() {
                        let request_hash = out_points
                            .get(i)
                            .unwrap()
                            .previous_output()
                            .tx_hash()
                            .unpack();
                        if let Some(callback) = self.listening_requests.remove(&request_hash) {
                            callback.send(hash.clone()).expect("clear callback");
                        }
                    }
                    break;
                }
            }
        }
        Ok(())
    }
}
