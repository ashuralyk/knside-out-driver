use ckb_types::bytes::Bytes;
use ckb_types::packed::{CellDep, Script};
use derive_more::Constructor;

use super::context::KoContextGlobalCell;

#[derive(Constructor)]
pub struct KoRequest {
    pub function_call: Bytes,
    pub inputs: Vec<(Script, Bytes)>,
    pub candidates: Vec<Script>,
    pub components: Vec<Bytes>,
    pub payment_ckb: u64,
    pub capacity: u64,
}

#[derive(Constructor)]
pub struct KoProject {
    pub cell_dep: CellDep,
    pub lua_code: Bytes,
    pub contract_owner: Script,
}

pub struct KoAssembleReceipt {
    pub requests: Vec<KoRequest>,
    pub global_cell: KoContextGlobalCell,
    pub random_seeds: [i64; 2],
}

impl KoAssembleReceipt {
    pub fn new(
        requests: Vec<KoRequest>,
        global_cell: KoContextGlobalCell,
        random_bytes: [u8; 16],
    ) -> Self {
        let random_seeds = {
            let mut seed_one = [0u8; 8];
            seed_one.copy_from_slice(&random_bytes[..8]);
            let mut seed_two = [0u8; 8];
            seed_two.copy_from_slice(&random_bytes[8..]);
            [i64::from_le_bytes(seed_one), i64::from_le_bytes(seed_two)]
        };
        KoAssembleReceipt {
            requests,
            global_cell,
            random_seeds,
        }
    }
}

#[derive(Constructor)]
pub struct KoCellOutput {
    pub cells: Vec<(Script, Option<Bytes>)>,
    pub capacity: u64,
}
