use crate::ckb_types::bytes::Bytes;
use crate::ckb_types::H256;
use crate::ckb_types::packed::{CellDep, Script};
use crate::derive_more::Constructor;

#[derive(Constructor)]
pub struct KoRequest {
    pub json_data: Bytes,
    pub function_call: Bytes,
    pub lock_script: Script,
    pub capacity: u64
}

#[derive(Constructor)]
pub struct KoProject {
    pub cell_dep: CellDep,
    pub lua_code: Bytes
}

#[derive(Constructor)]
pub struct KoAssembleReceipt {
    pub requests: Vec<KoRequest>,
    pub global_json_data: Bytes,
    pub project_owner: H256
}
