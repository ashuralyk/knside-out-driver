use ko_core_driver::DriverImpl;
use ko_protocol::ckb_jsonrpc_types::TransactionView as JsonTxView;
use ko_protocol::ckb_types::core::{DepType, TransactionView};
use ko_protocol::ckb_types::packed::WitnessArgs;
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack};
use ko_protocol::ckb_types::{bytes::Bytes, h256, H256};
use ko_protocol::secp256k1::SecretKey;
use ko_protocol::traits::{Backend, CkbClient, Driver};
use ko_protocol::types::config::KoCellDep;
use ko_protocol::{serde_json, tokio};
use ko_rpc_client::RpcClient;

use crate::BackendImpl;

const CKB_URL: &str = "http://127.0.0.1:8114";
const CKB_INDEXER_URL: &str = "http://127.0.0.1:8116";

const OWNER_PRIVATE_KEY: H256 =
    h256!("0x9a8fc5c463841c152800ec45ef4ceb03586177a7e6a9f34a6e40256310325e43");
const OWNER_ADDRESS: &str = "ckt1qyqycu3e597mvx7qpdpf45jdpn5u27w574rq8stzv3";

const PROJECT_CODE_HASH: H256 =
    h256!("0xab83b71e59d9c17cae16de47dba6c570bdccf7a81b42d149ee44f2d433c628c3");
const PROJECT_TYPE_ARGS: H256 =
    h256!("0x31f5d68df13196cc53f07f66b9c52fed15b8aadeda1b6e76319ddc3d7468c741");

const SECP256K1_TX_HASH: H256 =
    h256!("0x5c7b70f4fd242ff0fb703de908e2e7eef21621b640fe9a9c752643021a87bc1f");
const KNSIDEOUT_TX_HASH: H256 =
    h256!("0x6322ef6fb705e398cc6da4be08ced99f2c0ff6828e9246f0fb9c871ccf17973d");

async fn sign_and_push(rpc_client: &impl CkbClient, tx: TransactionView) {
    // sign transaction
    let privkey = SecretKey::from_slice(OWNER_PRIVATE_KEY.as_bytes()).unwrap();
    let driver = DriverImpl::new(rpc_client, &privkey);
    let signature = driver.sign_ko_transaction(&tx);

    // set witnesses
    let witness = WitnessArgs::new_builder()
        .lock(Some(signature).pack())
        .build()
        .as_bytes();
    let tx = tx
        .as_advanced_builder()
        .witnesses(vec![witness].pack())
        .build();
    println!(
        "tx = {}",
        serde_json::to_string_pretty(&JsonTxView::from(tx.clone())).unwrap()
    );

    // send knside-out transaction to CKB
    let hash = driver.send_ko_transaction(tx).await.expect("send tx");
    println!("send success, hash = {}", hash);
}

#[tokio::test]
async fn deploy_project_deployment_cell() {
    let lua_code = std::fs::read_to_string("./src/tests/21-point.lua").unwrap();
    let cell_deps = vec![
        KoCellDep::new(SECP256K1_TX_HASH.clone(), 0, DepType::DepGroup.into()),
        KoCellDep::new(KNSIDEOUT_TX_HASH.clone(), 0, DepType::Code.into()),
    ];

    // create digest
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let mut backend = BackendImpl::new(&rpc_client);
    let (digest, type_args) = backend
        .create_project_deploy_digest(
            Bytes::from(lua_code.as_bytes().to_vec()),
            OWNER_ADDRESS.into(),
            &PROJECT_CODE_HASH,
            &cell_deps,
        )
        .await
        .expect("create digest");
    println!("project_type_args = {}", type_args);

    // sign and push transaction
    let tx = backend.pop_transaction(&digest).await.expect("pop");
    sign_and_push(&rpc_client, tx).await;
}

#[tokio::test]
async fn update_project_deployment_cell() {
    let lua_code = std::fs::read_to_string("./src/tests/21-point.lua").unwrap();
    let cell_deps = vec![
        KoCellDep::new(SECP256K1_TX_HASH.clone(), 0, DepType::DepGroup.into()),
        KoCellDep::new(KNSIDEOUT_TX_HASH.clone(), 0, DepType::Code.into()),
    ];

    // create digest
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let mut backend = BackendImpl::new(&rpc_client);
    let digest = backend
        .create_project_update_digest(
            Bytes::from(lua_code.as_bytes().to_vec()),
            OWNER_ADDRESS.into(),
            &PROJECT_TYPE_ARGS,
            &cell_deps,
        )
        .await
        .expect("create digest");

    // sign and push transaction
    let tx = backend.pop_transaction(&digest).await.expect("pop");
    sign_and_push(&rpc_client, tx).await;
}

#[tokio::test]
async fn request_project_request_cell() {
    let cell_deps = vec![
        KoCellDep::new(SECP256K1_TX_HASH.clone(), 0, DepType::DepGroup.into()),
        KoCellDep::new(KNSIDEOUT_TX_HASH.clone(), 0, DepType::Code.into()),
    ];

    // create digest
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let mut backend = BackendImpl::new(&rpc_client);
    let digest = backend
        .create_project_request_digest(
            OWNER_ADDRESS.into(),
            None,
            None,
            "battle_win()".into(),
            &PROJECT_CODE_HASH,
            &PROJECT_TYPE_ARGS,
            &cell_deps,
        )
        .await
        .expect("create digest");

    // sign and push transaction
    let tx = backend.pop_transaction(&digest).await.expect("pop");
    sign_and_push(&rpc_client, tx).await;
}

#[tokio::test]
async fn fetch_global_json_data() {
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let backend = BackendImpl::new(&rpc_client);
    let global_data = backend
        .search_global_data(&PROJECT_CODE_HASH, &PROJECT_TYPE_ARGS)
        .await
        .expect("search global");
    println!("global_data = {}", global_data);
}

#[tokio::test]
async fn fetch_personal_json_data() {
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let backend = BackendImpl::new(&rpc_client);
    let personal_data = backend
        .search_personal_data(OWNER_ADDRESS.into(), &PROJECT_CODE_HASH, &PROJECT_TYPE_ARGS)
        .await
        .expect("search personal");
    personal_data.into_iter().for_each(|(data, outpoint)| {
        println!("personal_data = {}, outpoint = {}", data, outpoint);
    });
}
