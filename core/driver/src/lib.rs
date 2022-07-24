use std::vec;

use ckb_hash::new_blake2b;
use ckb_jsonrpc_types::TransactionView as JsonTxView;
use ko_protocol::ckb_sdk::{CkbRpcClient, SECP256K1};
use ko_protocol::ckb_types::packed::{Transaction, WitnessArgs};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack};
use ko_protocol::ckb_types::{bytes::Bytes, core::TransactionView, H256};
use ko_protocol::secp256k1::{Message, SecretKey};
use ko_protocol::serde_json::to_string_pretty;
use ko_protocol::traits::Driver;
use ko_protocol::{async_trait, tokio, KoResult};

mod error;
use error::DriverError;

pub struct DriverImpl {
    rpc_client: CkbRpcClient,
}

impl DriverImpl {
    pub fn new(ckb_url: &str) -> DriverImpl {
        DriverImpl {
            rpc_client: CkbRpcClient::new(ckb_url),
        }
    }
}

#[async_trait]
impl Driver for DriverImpl {
    async fn fetch_transactions_from_blocks_range(
        &self,
        begin_blocknumber: u64,
        end_blocknumber: u64,
    ) -> KoResult<Vec<TransactionView>> {
        let mut promises = vec![];
        for i in begin_blocknumber..(end_blocknumber + 1) {
            let mut rpc = CkbRpcClient::new(self.rpc_client.url.as_str());
            let handle = tokio::spawn(async move { rpc.get_block_by_number(i.into()) });
            promises.push((i, handle));
        }
        let mut txs = vec![];
        for (i, promise) in promises {
            let block = promise
                .await
                .unwrap()
                .map_err(|_| DriverError::InvalidBlockNumber(i))?;
            if let Some(block) = block {
                block
                    .transactions
                    .into_iter()
                    .for_each(|tx| txs.push(Transaction::from(tx.inner).into_view()));
            } else {
                return Err(DriverError::InvalidBlockNumber(i).into());
            }
        }
        Ok(txs)
    }

    fn sign_ko_transaction(&self, tx: &TransactionView, privkey: &SecretKey) -> Bytes {
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
        let signature = SECP256K1.sign_recoverable(&digest, privkey);
        let signature_bytes = {
            let (recover_id, signature) = signature.serialize_compact();
            let mut bytes = signature.to_vec();
            bytes.push(recover_id.to_i32() as u8);
            bytes
        };
        Bytes::from(signature_bytes)
    }

    fn send_ko_transaction(&mut self, tx: TransactionView) -> KoResult<H256> {
        let hash = self
            .rpc_client
            .send_transaction(tx.data().into(), None)
            .map_err(|err| {
                DriverError::TransactionSendError(
                    err.to_string(),
                    to_string_pretty(&JsonTxView::from(tx)).unwrap(),
                )
            })?;
        Ok(hash)
    }
}
