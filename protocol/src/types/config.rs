use ckb_types::packed::{CellDep, OutPoint};
use ckb_types::prelude::{Builder, Entity, Pack};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::H256;

#[derive(Deserialize, Clone, Constructor)]
pub struct KoCellDep {
    pub transaction_hash: H256,
    pub cell_index: u32,
    pub dep_type: u8,
}

impl From<&KoCellDep> for CellDep {
    fn from(cell_dep: &KoCellDep) -> Self {
        CellDep::new_builder()
            .out_point(OutPoint::new(
                cell_dep.transaction_hash.pack(),
                cell_dep.cell_index,
            ))
            .dep_type(cell_dep.dep_type.into())
            .build()
    }
}

#[derive(Deserialize, Clone, Constructor)]
pub struct KoDriveConfig {
    pub drive_interval_sec: u8,
    pub max_reqeusts_count: u8,
    pub block_confirms_count: u8,
    pub kickout_idle_sec: u64,
}

#[derive(Deserialize)]
pub struct KoConfig {
    pub project_manager_address: String,
    pub project_manager_privkey: H256,
    pub project_code_hash: H256,
    pub ckb_url: String,
    pub ckb_indexer_url: String,
    pub rpc_endpoint: String,
    pub persist_interval_sec: u64,
    pub project_cell_deps: Vec<KoCellDep>,
    pub drive_settings: KoDriveConfig,
}

impl AsRef<KoConfig> for KoConfig {
    fn as_ref(&self) -> &Self {
        self
    }
}

#[derive(Deserialize, Serialize, Constructor, Clone)]
pub struct KoTypeArgsItem {
    pub hash: H256,
    pub enable: bool,
}

#[derive(Deserialize, Serialize, Constructor)]
pub struct KoConfigTypeArgs {
    pub project_type_args: Vec<KoTypeArgsItem>,
}

impl From<KoConfigTypeArgs> for Vec<(H256, bool)> {
    fn from(config: KoConfigTypeArgs) -> Self {
        config
            .project_type_args
            .iter()
            .map(|v| (v.hash.clone(), v.enable))
            .collect()
    }
}
