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
    h256!("0x56a2ace407c9a1390896c2ddfd364c2eb56ce167a071206cf8c41f0fa5ed96a8");
const PROJECT_TYPE_ARGS: H256 =
    h256!("0x75c82f2165f1c6b90e2b6a5d33ce556f30daeaba3f855c0b81b2fa1f9fe0e4cd");

const SECP256K1_TX_HASH: H256 =
    h256!("0x5c7b70f4fd242ff0fb703de908e2e7eef21621b640fe9a9c752643021a87bc1f");
const KNSIDEOUT_TX_HASH: H256 =
    h256!("0xd08a5e45937c8dbe850f1bad6bdb3a613c06ca6cad9743905f42a4396ada785f");

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
