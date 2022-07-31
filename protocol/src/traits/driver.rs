use std::time::Duration;

use crate::types::config::KoCellDep;
use crate::{async_trait, KoResult};
use ckb_types::bytes::Bytes;
use ckb_types::core::TransactionView;
use ckb_types::packed::CellDep;
use ckb_types::H256;

#[async_trait]
pub trait Driver {
    async fn prepare_ko_transaction_normal_celldeps(
        &self,
        project_cell_deps: &[KoCellDep],
    ) -> KoResult<Vec<CellDep>>;

    fn sign_ko_transaction(&self, tx: &TransactionView) -> Bytes;

    async fn send_ko_transaction(&self, tx: TransactionView) -> KoResult<H256>;

    async fn wait_ko_transaction_committed(&self, hash: &H256, interval: &Duration)
        -> KoResult<()>;
}
