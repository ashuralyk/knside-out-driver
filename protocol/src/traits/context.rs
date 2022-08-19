use crate::{async_trait, KoResult};
use ckb_types::packed::{CellDep, Script};
use ckb_types::H256;
use tokio::sync::mpsc::UnboundedSender;

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

    fn listen_request_committed(
        &mut self,
        request_hash: &H256,
        sender: UnboundedSender<KoResult<H256>>,
    );

    async fn run(mut self, project_cell_deps: &[CellDep]);
}
