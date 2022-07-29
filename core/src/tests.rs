use ko_protocol::traits::Assembler;
use ko_rpc_client::RpcClient;
use ko_core_assembler::AssemblerImpl;
use ko_core_driver::DriverImpl;
use ko_core_executor::ExecutorImpl;
use ko_protocol::secp256k1::SecretKey;
use ko_protocol::ckb_types::{h256, H256, core::DepType};
use ko_protocol::ckb_types::packed::{CellDep, OutPoint};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack};
use ko_protocol::tokio;

use crate::Context;

const CKB_URL: &str = "http://127.0.0.1:8114";
const CKB_INDEXER_URL: &str = "http://127.0.0.1:8116";

const OWNER_PRIVATE_KEY: H256 =
    h256!("0x9a8fc5c463841c152800ec45ef4ceb03586177a7e6a9f34a6e40256310325e43");

const PROJECT_CODE_HASH: H256 =
    h256!("0xe72d02d5a2dbf1b893e5b9ae17602395533b6b918cee67d234669c24b41cc4f7");
const PROJECT_TYPE_ARGS: H256 =
    h256!("0x8ee3bccb2a5b2cc0a04bdcef804ac39bb395dd64207c840c5764504d526c6d34");

const SECP256K1_TX_HASH: H256 =
    h256!("0x5c7b70f4fd242ff0fb703de908e2e7eef21621b640fe9a9c752643021a87bc1f");
const KNSIDEOUT_TX_HASH: H256 =
    h256!("0x6bbbd9a777fe6e115de76c01dac97afd24a1c54c0ebd06f0eb1a6bb42e512ead");

#[tokio::test]
async fn drive_one() {
    // prepare parts
    let rpc_client = RpcClient::new(&CKB_URL, &CKB_INDEXER_URL);
    let assembler = AssemblerImpl::new(
        &rpc_client,
        &PROJECT_TYPE_ARGS,
        &PROJECT_CODE_HASH,
    );
    let privkey = SecretKey::from_slice(OWNER_PRIVATE_KEY.as_bytes()).unwrap();
    let driver = DriverImpl::new(&rpc_client, &privkey);
    let executor = ExecutorImpl::new();

    // prepare to make instance of context
    let transaction_deps = vec![
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
    let lua_code = assembler
        .prepare_ko_transaction_project_celldep()
        .await
        .expect("project dep")
        .lua_code;

    // drive knside-out transaction
    let ctx = Context::new(assembler, executor, driver);
    let hash = ctx
        .drive(&lua_code, &transaction_deps)
        .await
        .expect("drive");
    println!("hash = {:?}", hash);
}
