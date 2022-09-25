use ckb_types::packed::Script;
use tokio::sync::mpsc::UnboundedSender;

use crate::{async_trait, KoResult, H256};

#[async_trait]
pub trait ContextRpc: Send + Sync {
    async fn start_project_driver(&mut self, project_type_args: &H256) -> bool;

    async fn estimate_payment_ckb(
        &mut self,
        project_type_args: &H256,
        sender: &Script,
        method_call: &str,
        previous_json_data: &str,
        recipient: &Option<Script>,
        response: UnboundedSender<KoResult<u64>>,
    ) -> bool;

    async fn listen_request_committed(
        &mut self,
        project_type_args: &H256,
        request_hash: &H256,
        response: UnboundedSender<KoResult<H256>>,
    ) -> bool;
}
