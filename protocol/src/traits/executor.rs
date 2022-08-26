use crate::ckb_types::bytes::Bytes;
use crate::ckb_types::H256;
use crate::types::assembler::KoRequest;
use crate::types::executor::KoExecuteReceipt;
use crate::KoResult;

pub trait Executor {
    fn execute_lua_requests(
        &self,
        global_json_data: &Bytes,
        project_owner: &H256,
        user_requests: &[KoRequest],
        project_lua_code: &Bytes,
        random_seeds: &[i64; 2],
    ) -> KoResult<KoExecuteReceipt>;

    fn estimate_payment_ckb(
        &self,
        global_json_data: &Bytes,
        project_owner: &H256,
        request: KoRequest,
        project_lua_code: &Bytes,
    ) -> KoResult<u64>;
}
