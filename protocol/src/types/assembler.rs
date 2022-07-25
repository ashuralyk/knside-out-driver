use ckb_types::bytes::Bytes;
use ckb_types::packed::{CellDep, Script};
use ckb_types::prelude::{Entity, Builder};
use derive_more::Constructor;

#[derive(Constructor)]
pub struct KoRequest {
    pub json_data: Bytes,
    pub function_call: Bytes,
    pub lock_script: Script,
    pub payment: u64
}

#[derive(Constructor)]
pub struct KoProject {
    pub cell_dep: CellDep,
    pub lua_code: Bytes
}

impl Default for KoProject {
    fn default() -> Self {
        KoProject {
            cell_dep: CellDep::new_builder().build(),
            lua_code: Bytes::default()
        }
    }
}

#[derive(Constructor)]
pub struct KoAssembleReceipt {
    pub requests: Vec<KoRequest>,
    pub global_json_data: Bytes,
    pub global_lockscript: Script,
    pub total_inputs_capacity: u64
}

#[derive(Constructor)]
pub struct KoCellOutput {
    pub data: Bytes,
    pub lock_script: Script,
    pub payment: u64
}
