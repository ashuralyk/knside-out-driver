pub mod traits;
pub mod types;

pub use async_trait::async_trait;
pub use ckb_jsonrpc_types;
pub use ckb_sdk;
pub use ckb_types;
pub use derive_more;
pub use hex;
pub use lazy_static::lazy_static;
pub use secp256k1;
pub use serde_json;
pub use tokio;

pub use types::error::KoResult;
pub use types::generated::*;

use ckb_types::{packed::CellDep, H256};
use types::config::KoCellDep;

#[derive(Clone)]
pub struct ProjectDeps {
    pub project_code_hash: H256,
    pub project_type_args: H256,
    pub project_cell_deps: Vec<CellDep>,
}

impl ProjectDeps {
    pub fn new(code_hash: &H256, type_args: &H256, cell_deps: &[KoCellDep]) -> Self {
        let cell_deps = cell_deps.iter().map(|dep| dep.into()).collect::<Vec<_>>();
        ProjectDeps {
            project_code_hash: code_hash.clone(),
            project_type_args: type_args.clone(),
            project_cell_deps: cell_deps,
        }
    }
}

impl From<&types::config::KoConfig> for ProjectDeps {
    fn from(config: &types::config::KoConfig) -> Self {
        ProjectDeps {
            project_code_hash: config.project_code_hash.clone(),
            project_type_args: config.project_type_args.clone(),
            project_cell_deps: config
                .project_cell_deps
                .iter()
                .map(|v| v.into())
                .collect::<Vec<_>>(),
        }
    }
}

#[allow(non_snake_case)]
pub mod TestVars {
    use crate::ckb_types::{core::DepType, h256, H256};
    use crate::types::config::KoCellDep;
    use crate::{lazy_static, ProjectDeps};

    pub const CKB_URL: &str = "http://127.0.0.1:8114";
    pub const CKB_INDEXER_URL: &str = "http://127.0.0.1:8116";

    pub const OWNER_PRIVATE_KEY: H256 =
        h256!("0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df");
    pub const OWNER_ADDRESS: &str = "ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl";

    pub const PROJECT_CODE_HASH: H256 =
        h256!("0x8f11b7a80bb50a518cd29170cbc72b2ab8ef94fc8297f75cfa9f8917d5057e4b");
    pub const PROJECT_TYPE_ARGS: H256 =
        h256!("0xdd53c2bb4be8102693feb11b8325cbd69b1265cb5f215c6cfff5fefbaede50ba");

    pub const SECP256K1_TX_HASH: H256 =
        h256!("0xc6bffa9ca9a9dadfec83c0307eee18fe88e42a00d05068510d799e3e4ad3ee87");
    pub const KNSIDEOUT_TX_HASH: H256 =
        h256!("0xb769ef9ccc18b5a883a4a44f31217036458be38dbef874a10e69d8f5b0f322a0");

    lazy_static! {
        pub static ref PROJECT_VARS: ProjectDeps = ProjectDeps::new(
            &PROJECT_CODE_HASH,
            &PROJECT_TYPE_ARGS,
            &[
                KoCellDep::new(SECP256K1_TX_HASH.clone(), 0, DepType::DepGroup.into()),
                KoCellDep::new(KNSIDEOUT_TX_HASH.clone(), 0, DepType::Code.into()),
            ]
        );
    }
}
