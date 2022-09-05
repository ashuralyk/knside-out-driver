use ckb_types::packed::Script;
use tokio::sync::mpsc::UnboundedSender;

use crate::{KoResult, H256};

pub trait ContextRpc: Send + Sync {
    fn start_project_driver(&mut self, project_type_args: &H256) -> bool;

    fn estimate_payment_ckb(
        &self,
        project_type_args: &H256,
        sender: &Script,
        method_call: &str,
        previous_json_data: &str,
        recipient: &Option<Script>,
        response: UnboundedSender<KoResult<u64>>,
    ) -> bool;

    fn listen_request_committed(
        &self,
        project_type_args: &H256,
        request_hash: &H256,
        response: UnboundedSender<KoResult<H256>>,
    ) -> bool;
}
