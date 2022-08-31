use ko_protocol::ckb_types::{packed::Script, H256};
use ko_protocol::tokio::sync::mpsc::UnboundedSender;
use ko_protocol::traits::ContextRpc;
use ko_protocol::KoResult;

#[derive(Default, Clone, Copy)]
pub struct MockContextrpc {}

impl ContextRpc for MockContextrpc {
    fn start_project_driver(&mut self, _project_type_args: &H256) -> bool {
        false
    }

    fn estimate_payment_ckb(
        &self,
        _project_type_args: &H256,
        _sender: &Script,
        _method_call: &str,
        _previous_json_data: &str,
        _recipient: &Option<Script>,
        _response: UnboundedSender<KoResult<u64>>,
    ) -> bool {
        false
    }

    fn listen_request_committed(
        &self,
        _project_type_args: &H256,
        _request_hash: &H256,
        _response: UnboundedSender<KoResult<H256>>,
    ) -> bool {
        false
    }
}
