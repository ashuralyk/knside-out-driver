use ko_core_assembler::AssemblerImpl;
use ko_core_driver::DriverImpl;
use ko_core_executor::ExecutorImpl;
use ko_protocol::ckb_types::packed::{CellDep, OutPoint};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack};
use ko_protocol::ckb_types::{core::DepType, h256, H256};
use ko_protocol::secp256k1::SecretKey;
use ko_protocol::tokio;
use ko_protocol::traits::Assembler;
use ko_rpc_client::RpcClient;

use crate::Context;

const CKB_URL: &str = "http://127.0.0.1:8114";
const CKB_INDEXER_URL: &str = "http://127.0.0.1:8116";

const OWNER_PRIVATE_KEY: H256 =
    h256!("0x9a8fc5c463841c152800ec45ef4ceb03586177a7e6a9f34a6e40256310325e43");

const PROJECT_CODE_HASH: H256 =
    h256!("0xab83b71e59d9c17cae16de47dba6c570bdccf7a81b42d149ee44f2d433c628c3");
const PROJECT_TYPE_ARGS: H256 =
    h256!("0x31f5d68df13196cc53f07f66b9c52fed15b8aadeda1b6e76319ddc3d7468c741");

const SECP256K1_TX_HASH: H256 =
    h256!("0x5c7b70f4fd242ff0fb703de908e2e7eef21621b640fe9a9c752643021a87bc1f");
const KNSIDEOUT_TX_HASH: H256 =
    h256!("0x6322ef6fb705e398cc6da4be08ced99f2c0ff6828e9246f0fb9c871ccf17973d");

#[tokio::test]
async fn drive_one() {
    // prepare parts
    let rpc_client = RpcClient::new(&CKB_URL, &CKB_INDEXER_URL);
    let assembler = AssemblerImpl::new(&rpc_client, &PROJECT_TYPE_ARGS, &PROJECT_CODE_HASH);
    let privkey = SecretKey::from_slice(OWNER_PRIVATE_KEY.as_bytes()).unwrap();
    let driver = DriverImpl::new(&rpc_client, &privkey);
    let executor = ExecutorImpl::new();

    // prepare to make instance of context
    let project_dep = assembler
        .prepare_ko_transaction_project_celldep()
        .await
        .expect("project dep");
    let mut transaction_deps = vec![
        (SECP256K1_TX_HASH.clone(), 0, DepType::DepGroup),
        (KNSIDEOUT_TX_HASH.clone(), 0, DepType::Code),
    ]
    .into_iter()
    .map(|(tx_hash, index, dep_type)| {
        CellDep::new_builder()
            .out_point(OutPoint::new(tx_hash.pack(), index))
            .dep_type(dep_type.into())
            .build()
    })
    .collect::<Vec<_>>();
    transaction_deps.insert(0, project_dep.cell_dep);

    // drive knside-out transaction
    let ctx = Context::new(assembler, executor, driver);
    let hash = ctx
        .drive(&project_dep.lua_code, &transaction_deps)
        .await
        .expect("drive");
    println!("hash = {}", hex::encode(&hash.unwrap_or(H256::default())));
}
