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
        h256!("0x9a8fc5c463841c152800ec45ef4ceb03586177a7e6a9f34a6e40256310325e43");
    pub const OWNER_ADDRESS: &str = "ckt1qyqycu3e597mvx7qpdpf45jdpn5u27w574rq8stzv3";

    pub const PROJECT_CODE_HASH: H256 =
        h256!("0x0883e9527e2798d7bb3540b1186297464fdfb71bf59566971b0824c781aaa6c0");
    pub const PROJECT_TYPE_ARGS: H256 =
        h256!("0xd6568eda1c20e30b41cd15be2f9ab8db9446561097ee801cafabdb6ca6133e05");

    pub const SECP256K1_TX_HASH: H256 =
        h256!("0x5c7b70f4fd242ff0fb703de908e2e7eef21621b640fe9a9c752643021a87bc1f");
    pub const KNSIDEOUT_TX_HASH: H256 =
        h256!("0xb88a68436c16dbdfbd5d3c3e38c5dcd4905514e0c8ead8e0b1b8533bc63d32e0");
}
