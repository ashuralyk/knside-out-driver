use crate::KoResult;
use crate::types::config::KoCellDep;
use ckb_types::bytes::Bytes;
use ckb_types::core::TransactionView;
use ckb_types::packed::CellDep;
use ckb_types::H256;

pub trait Driver {
    fn prepare_ko_transaction_normal_celldeps(
        &mut self,
        project_cell_deps: &Vec<KoCellDep>,
    ) -> KoResult<Vec<CellDep>>;

    fn sign_ko_transaction(&self, tx: &TransactionView) -> Bytes;

    fn send_ko_transaction(&mut self, tx: TransactionView) -> KoResult<H256>;
}
