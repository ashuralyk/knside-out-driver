use ckb_types::bytes::Bytes;
use ckb_types::packed::Script;

use crate::types::assembler::KoRequest;
use crate::types::context::KoContextGlobalCell;
use crate::types::executor::KoExecutedRequest;
use crate::KoResult;

pub trait Executor {
    fn execute_lua_requests(
        &self,
        global_cell: &mut KoContextGlobalCell,
        project_owner: &Script,
        user_requests: &[KoRequest],
        project_lua_code: &Bytes,
        random_seeds: &[i64; 2],
    ) -> KoResult<Vec<KoExecutedRequest>>;

    fn estimate_payment_ckb(
        &self,
        global_cell: &KoContextGlobalCell,
        project_owner: &Script,
        request: KoRequest,
        project_lua_code: &Bytes,
    ) -> KoResult<u64>;
}
