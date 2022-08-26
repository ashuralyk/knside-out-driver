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
pub use serde;
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

    pub const CKB_URL: &str = "http://127.0.0.1:8114/";
    pub const CKB_INDEXER_URL: &str = "http://127.0.0.1:8116/";

    pub const OWNER_PRIVATE_KEY: H256 =
        h256!("0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df");
    pub const OWNER_ADDRESS: &str = "ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl";

    pub const PROJECT_CODE_HASH: H256 =
        h256!("0x8680e03788eb830d44821e9e9cacfc3a30d38d48f2e439ab4e861d765454bd16");

    // testnet
    // pub const PROJECT_TYPE_ARGS: H256 =
    //     h256!("0x3fd9221c7ca05c98b3bd8247adf3291212ed1663a131752174981e98f994da4d");
    // pub const SECP256K1_TX_HASH: H256 =
    //     h256!("0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37");
    // pub const KNSIDEOUT_TX_HASH: H256 =
    //     h256!("0x2bec96b9d22f3c72ad75423395aa8d5ad3881cf13bfbf1ffbf8a4bd7994621e7");

    // devnet
    pub const PROJECT_TYPE_ARGS: H256 =
        h256!("0xa033c093798f0ee671b70c44edb6a4825339fd297c1b516b80eea56e78c95e42");
    pub const SECP256K1_TX_HASH: H256 =
        h256!("0xc6bffa9ca9a9dadfec83c0307eee18fe88e42a00d05068510d799e3e4ad3ee87");
    pub const KNSIDEOUT_TX_HASH: H256 =
        h256!("0x54c0774ffc8330216a8a8363f6b327c526d15be30830e55da3516c3913a5b413");

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
