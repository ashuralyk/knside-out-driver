use crate::KoResult;
use crate::types::assembler::KoRequest;
use crate::types::executor::KoExecuteReceipt;
use crate::ckb_types::H256;
use crate::ckb_types::bytes::Bytes;

pub trait Executor {
    fn execute_lua_requests (
        &self,
        global_json_data: &Bytes,
        project_owner: &H256,
        user_requests: &Vec<KoRequest>,
        project_lua_code: &Bytes
    ) -> KoResult<KoExecuteReceipt>;
}
