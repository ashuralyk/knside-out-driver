use ko_core_assembler::AssemblerImpl;
use ko_core_driver::DriverImpl;
use ko_core_executor::ExecutorImpl;
use ko_protocol::ckb_types::packed::{CellDep, OutPoint};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack};
use ko_protocol::ckb_types::{core::DepType, H256};
use ko_protocol::secp256k1::SecretKey;
use ko_protocol::traits::Assembler;
use ko_protocol::{hex, tokio, TestVars::*};
use ko_rpc_client::RpcClient;

use crate::Context;

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
