use ckb_types::bytes::Bytes;
use ckb_types::packed::{CellDep, Script};
use derive_more::Constructor;

#[derive(Constructor)]
pub struct KoRequest {
    pub json_data: Bytes,
    pub function_call: Bytes,
    pub lock_script: Script,
    pub recipient_script: Option<Script>,
    pub payment: u64,
    pub ckb: u64,
}

#[derive(Constructor)]
pub struct KoProject {
    pub cell_dep: CellDep,
    pub lua_code: Bytes,
}

pub struct KoAssembleReceipt {
    pub requests: Vec<KoRequest>,
    pub global_json_data: Bytes,
    pub global_lockscript: Script,
    pub global_ckb: u64,
    pub random_seeds: [u64; 2],
}

impl KoAssembleReceipt {
    pub fn new(
        requests: Vec<KoRequest>,
        global_json_data: Bytes,
        global_lockscript: Script,
        global_ckb: u64,
        random_bytes: [u8; 16],
    ) -> Self {
        let random_seeds = {
            let mut seed_one = [0u8; 8];
            seed_one.copy_from_slice(&random_bytes[..8]);
            let mut seed_two = [0u8; 8];
            seed_two.copy_from_slice(&random_bytes[8..]);
            [u64::from_le_bytes(seed_one), u64::from_le_bytes(seed_two)]
        };
        KoAssembleReceipt {
            requests,
            global_json_data,
            global_lockscript,
            global_ckb,
            random_seeds,
        }
    }
}

#[derive(Constructor)]
pub struct KoCellOutput {
    pub data: Option<Bytes>,
    pub lock_script: Script,
    pub extra_capacity: u64,
}
