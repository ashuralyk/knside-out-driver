pub mod traits;
pub mod types;

use std::convert::TryFrom;
use std::str::FromStr;

pub use async_trait::async_trait;
pub use ckb_jsonrpc_types;
pub use ckb_sdk;
pub use ckb_types;
pub use derive_more;
pub use hex;
pub use lazy_static::lazy_static;
pub use log;
pub use secp256k1;
pub use serde;
pub use serde_json;
pub use tokio;

pub use types::error::KoResult;
pub use types::generated::*;
pub use types::h256::H256;

use ckb_sdk::Address;
use ckb_types::packed::CellDep;
use log::{Level, Log, Metadata, Record};
use types::config::KoCellDep;

#[derive(Clone)]
pub struct ProjectDeps {
    pub project_manager: Address,
    pub project_code_hash: H256,
    pub project_cell_deps: Vec<CellDep>,
}

impl ProjectDeps {
    pub fn new(code_hash: &H256, manager_address: &Address, cell_deps: &[KoCellDep]) -> Self {
        let cell_deps = cell_deps.iter().map(|dep| dep.into()).collect::<Vec<_>>();
        ProjectDeps {
            project_manager: manager_address.clone(),
            project_code_hash: code_hash.clone(),
            project_cell_deps: cell_deps,
        }
    }
}

impl TryFrom<&types::config::KoConfig> for ProjectDeps {
    type Error = String;

    fn try_from(config: &types::config::KoConfig) -> Result<Self, Self::Error> {
        match Address::from_str(&config.project_manager_address) {
            Ok(address) => Ok(ProjectDeps {
                project_manager: address,
                project_code_hash: config.project_code_hash.clone(),
                project_cell_deps: config
                    .project_cell_deps
                    .iter()
                    .map(|v| v.into())
                    .collect::<Vec<_>>(),
            }),
            Err(err) => Err(err),
        }
    }
}

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        if metadata.target().starts_with("ko_") {
            metadata.level() <= Level::Debug
        } else {
            false
        }
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{: >5} -> {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

#[allow(non_snake_case)]
pub mod TestVars {
    use crate::ckb_sdk::Address;
    use crate::ckb_types::{core::DepType, h256, H256};
    use crate::types::config::{KoCellDep, KoDriveConfig};
    use crate::{lazy_static, ProjectDeps};
    use std::str::FromStr;

    pub const CKB_URL: &str = "http://127.0.0.1:8114/";
    pub const CKB_INDEXER_URL: &str = "http://127.0.0.1:8116/";

    pub const OWNER_PRIVATE_KEY: H256 =
        h256!("0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df");
    pub const OWNER_ADDRESS: &str = "ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl";

    pub const PROJECT_CODE_HASH: H256 =
        h256!("0x05d0e558c42c8f52d0addc2dee8dda669b66637650c4e8a0c3845c5c1f395ece");

    // testnet
    // pub const PROJECT_TYPE_ARGS: H256 =
    //     h256!("0x3fd9221c7ca05c98b3bd8247adf3291212ed1663a131752174981e98f994da4d");
    // pub const SECP256K1_TX_HASH: H256 =
    //     h256!("0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37");
    // pub const KNSIDEOUT_TX_HASH: H256 =
    //     h256!("0x2bec96b9d22f3c72ad75423395aa8d5ad3881cf13bfbf1ffbf8a4bd7994621e7");

    // devnet
    pub const PROJECT_TYPE_ARGS: H256 =
        h256!("0xfc03b799cd921255f48aaf28f36d613d8addfd8b3dadbc945d94f21a3d00a67b");
    pub const SECP256K1_TX_HASH: H256 =
        h256!("0xc6bffa9ca9a9dadfec83c0307eee18fe88e42a00d05068510d799e3e4ad3ee87");
    pub const KNSIDEOUT_TX_HASH: H256 =
        h256!("0x1bb506c8e1f5a57f22ccd97e5f7f5624c87ddd7772076cf2d551250af85c19ca");

    lazy_static! {
        pub static ref PROJECT_VARS: ProjectDeps = ProjectDeps::new(
            &PROJECT_CODE_HASH.into(),
            &Address::from_str(OWNER_ADDRESS).unwrap(),
            &[
                KoCellDep::new(SECP256K1_TX_HASH.into(), 0, DepType::DepGroup.into()),
                KoCellDep::new(KNSIDEOUT_TX_HASH.into(), 0, DepType::Code.into()),
            ]
        );
        pub static ref DRIVE_CONFIG: KoDriveConfig = KoDriveConfig::new(3, 10, 3, 100);
    }

    #[derive(Default, Clone, Copy)]
    pub struct MockContextRpc {}
}

#[async_trait]
impl traits::ContextRpc for TestVars::MockContextRpc {
    async fn start_project_driver(&mut self, _project_type_args: &H256) -> bool {
        false
    }

    async fn estimate_payment_ckb(
        &mut self,
        _project_type_args: &H256,
        _method_call: &str,
        _inputs: &[(ckb_types::packed::Script, String)],
        _candidates: &[ckb_types::packed::Script],
        _components: &[String],
        _response: tokio::sync::mpsc::UnboundedSender<KoResult<u64>>,
    ) -> bool {
        false
    }

    async fn listen_request_committed(
        &mut self,
        _project_type_args: &H256,
        _request_hash: &H256,
        _response: tokio::sync::mpsc::UnboundedSender<KoResult<H256>>,
    ) -> bool {
        false
    }
}
