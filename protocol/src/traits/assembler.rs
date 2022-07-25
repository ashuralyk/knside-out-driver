use ckb_types::{bytes::Bytes, core::TransactionView};
use ckb_types::packed::CellDep;

use crate::types::assembler::{KoAssembleReceipt, KoProject, KoCellOutput};
use crate::KoResult;

pub trait Assembler {
    fn prepare_ko_transaction_project_celldep(
        &mut self,
        project_deployment_args: &[u8; 32]
    ) -> KoResult<KoProject>;

    fn generate_ko_transaction_with_inputs_and_celldeps(
        &mut self,
        cell_number: u8,
        cell_deps: &Vec<CellDep>
    ) -> KoResult<(TransactionView, KoAssembleReceipt)>;

    fn fill_ko_transaction_with_outputs(
        &self,
        tx: TransactionView,
        cell_outputs: &Vec<KoCellOutput>,
        inputs_capacity: u64,
    ) -> KoResult<TransactionView>;

    fn complete_ko_transaction_with_signature(
        &self,
        tx: TransactionView,
        signature: Bytes
    ) -> TransactionView;
}
