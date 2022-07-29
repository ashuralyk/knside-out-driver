use ckb_types::H256;
use ckb_types::packed::{CellDep, OutPoint};
use ckb_types::prelude::{Builder, Entity, Pack};
use serde::Deserialize;
use derive_more::Constructor;

#[derive(Deserialize, Clone, Constructor)]
pub struct KoCellDep {
    pub transaction_hash: H256,
    pub cell_index: u32,
    pub dep_type: u8,
}

impl From<&KoCellDep> for CellDep {
    fn from(cell_dep: &KoCellDep) -> Self {
        CellDep::new_builder()
            .out_point(OutPoint::new(cell_dep.transaction_hash.pack(), cell_dep.cell_index))
            .dep_type(cell_dep.dep_type.into())
            .build()
    }
}

#[derive(Deserialize)]
pub struct KoConfig {
    pub project_type_args: H256,
    pub project_owner_privkey: H256,
    pub project_code_hash: H256,
    pub ckb_url: String,
    pub ckb_indexer_url: String,
    pub project_cell_deps: Vec<KoCellDep>,
}
