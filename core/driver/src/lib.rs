use std::vec;

use ckb_hash::new_blake2b;
use ckb_jsonrpc_types::TransactionView as JsonTxView;
use ko_protocol::ckb_sdk::{CkbRpcClient, SECP256K1};
use ko_protocol::ckb_types::packed::{CellDep, OutPoint, WitnessArgs};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack};
use ko_protocol::ckb_types::{bytes::Bytes, core::TransactionView, H256};
use ko_protocol::secp256k1::{Message, SecretKey};
use ko_protocol::serde_json::to_string_pretty;
use ko_protocol::traits::Driver;
use ko_protocol::KoResult;

mod error;
use error::DriverError;

pub struct DriverImpl {
    rpc_client: CkbRpcClient,
    privkey: SecretKey,
}

impl DriverImpl {
    pub fn new(ckb_url: &str, privkey: &SecretKey) -> DriverImpl {
        DriverImpl {
            rpc_client: CkbRpcClient::new(ckb_url),
            privkey: privkey.clone(),
        }
    }
}

impl Driver for DriverImpl {
    fn prepare_ko_transaction_normal_celldeps(
        &mut self,
        project_cell_deps: &Vec<(H256, u32)>,
    ) -> KoResult<Vec<CellDep>> {
        let cell_deps = project_cell_deps
            .iter()
            .map(|(hash, index)| {
                self.rpc_client
                    .get_transaction(hash.clone())
                    .map_err(|err| {
                        DriverError::ErrorFetchingCelldepTransaction(err.to_string(), hash.clone())
                    })?;
                let cell_dep = CellDep::new_builder()
                    .out_point(OutPoint::new(hash.pack(), *index))
                    .build();
                Ok(cell_dep)
            })
            .collect::<KoResult<Vec<_>>>()?;
        Ok(cell_deps)
    }

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
