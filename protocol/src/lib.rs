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

#[allow(non_snake_case)]
pub mod TestVars {
    use crate::ckb_types::{h256, H256};

    pub const CKB_URL: &str = "http://127.0.0.1:8114";
    pub const CKB_INDEXER_URL: &str = "http://127.0.0.1:8116";

    pub const OWNER_PRIVATE_KEY: H256 =
        h256!("0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df");
    pub const OWNER_ADDRESS: &str = "ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl";

    pub const PROJECT_CODE_HASH: H256 =
        h256!("0x0883e9527e2798d7bb3540b1186297464fdfb71bf59566971b0824c781aaa6c0");
    pub const PROJECT_TYPE_ARGS: H256 =
        h256!("0x8f160104a98392cc0ca7d6d4c3da92e6e326810fd117397f39b9b2ec7cc3217c");

    pub const SECP256K1_TX_HASH: H256 =
        h256!("0xc6bffa9ca9a9dadfec83c0307eee18fe88e42a00d05068510d799e3e4ad3ee87");
    pub const KNSIDEOUT_TX_HASH: H256 =
        h256!("0xc9cc025d89bf5adae1362367cb3fd63fbbfb11a240f31448d021f3e591bf5a55");
}
