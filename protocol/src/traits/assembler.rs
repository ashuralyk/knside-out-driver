use crate::ckb_types::{bytes::Bytes, core::TransactionView};
use crate::ckb_types::packed::{Script, CellDep};
use crate::types::assembler::{KoAssembleReceipt, KoProject};
use crate::KoResult;

pub trait Assembler {
    fn prepare_ko_transaction_project_celldep(
        &mut self,
        project_deployment_args: &[u8; 32]
    ) -> KoResult<KoProject>;

    fn generate_ko_transaction_with_inputs_and_celldeps(
        &mut self,
        txs: &Vec<TransactionView>,
        cell_deps: &Vec<CellDep>
    ) -> KoResult<(TransactionView, KoAssembleReceipt)>;

    fn fill_ko_transaction_with_outputs(
        &self,
        tx: TransactionView,
        outputs_data: &Vec<Bytes>,
        inputs_capacity: u64,
        lock_scripts: &Vec<Script>,
    ) -> KoResult<TransactionView>;

    fn complete_ko_transaction_with_signature(
        &self,
        tx: TransactionView,
        signature: Bytes
    ) -> TransactionView;
}
