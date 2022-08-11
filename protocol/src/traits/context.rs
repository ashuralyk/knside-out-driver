use crate::{async_trait, KoResult};
use ckb_types::packed::{CellDep, Script};
use ckb_types::H256;

#[async_trait]
pub trait Context: Send + Sync {
    fn get_project_id(&self) -> H256;

    fn estimate_payment_ckb(
        &self,
        sender: &Script,
        method_call: &str,
        previous_json_data: &str,
        recipient: &Option<Script>,
    ) -> KoResult<u64>;

    async fn run(mut self, project_cell_deps: &[CellDep]);
}
