use std::time::Duration;

use crate::{async_trait, KoResult};
use ckb_types::bytes::Bytes;
use ckb_types::core::TransactionView;
use ckb_types::H256;

#[async_trait]
pub trait Driver {
    fn sign_ko_transaction(&self, tx: &TransactionView) -> Bytes;

    async fn send_ko_transaction(&self, tx: TransactionView) -> KoResult<H256>;

    async fn wait_ko_transaction_committed(
        &self,
        hash: &H256,
        interval: &Duration,
        confirms: u8,
    ) -> KoResult<()>;
}
