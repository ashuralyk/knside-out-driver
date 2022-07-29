use ckb_types::packed::CellDep;
use ckb_types::{bytes::Bytes, core::TransactionView};

use crate::types::assembler::{KoAssembleReceipt, KoCellOutput, KoProject};
use crate::{async_trait, KoResult};

#[async_trait]
pub trait Assembler {
    async fn prepare_ko_transaction_project_celldep(&self) -> KoResult<KoProject>;

    async fn generate_ko_transaction_with_inputs_and_celldeps(
        &self,
        cell_number: u8,
        cell_deps: &[CellDep],
    ) -> KoResult<(TransactionView, KoAssembleReceipt)>;

    async fn fill_ko_transaction_with_outputs(
        &self,
        tx: TransactionView,
        cell_outputs: &[KoCellOutput],
        inputs_capacity: u64,
    ) -> KoResult<TransactionView>;

    fn complete_ko_transaction_with_signature(
        &self,
        tx: TransactionView,
        signature: Bytes,
    ) -> TransactionView;
}
