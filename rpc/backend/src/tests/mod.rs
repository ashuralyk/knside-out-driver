use ko_core_driver::DriverImpl;
use ko_protocol::ckb_jsonrpc_types::TransactionView as JsonTxView;
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::core::{DepType, TransactionView};
use ko_protocol::ckb_types::packed::WitnessArgs;
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack};
use ko_protocol::secp256k1::SecretKey;
use ko_protocol::traits::{Backend, CkbClient, Driver};
use ko_protocol::types::config::KoCellDep;
use ko_protocol::{serde_json, tokio, TestVars::*};
use ko_rpc_client::RpcClient;

use crate::BackendImpl;

fn sign(rpc_client: &impl CkbClient, tx: TransactionView) -> [u8; 65] {
    println!(
        "tx = {}",
        serde_json::to_string_pretty(&JsonTxView::from(tx.clone())).unwrap()
    );
    // sign transaction
    let privkey = SecretKey::from_slice(OWNER_PRIVATE_KEY.as_bytes()).unwrap();
    let driver = DriverImpl::new(rpc_client, &privkey);

    {
        let mut bytes = [0u8; 65];
        let signature = driver.sign_ko_transaction(&tx);
        bytes.copy_from_slice(&signature);
        bytes
    }
}

#[allow(unused)]
async fn sign_and_push(rpc_client: &impl CkbClient, tx: TransactionView) {
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
    let tx = backend.peak_transaction(&digest).expect("peak");
    let signature = sign(&rpc_client, tx);
    let hash = backend
        .send_transaction_to_ckb(&digest, &signature)
        .await
        .expect("send")
        .unwrap();
    println!("send success, hash = {}", hash);
    // sign_and_push(&rpc_client, tx).await;
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
    let tx = backend.peak_transaction(&digest).expect("peak");
    let signature = sign(&rpc_client, tx);
    let hash = backend
        .send_transaction_to_ckb(&digest, &signature)
        .await
        .expect("send")
        .unwrap();
    println!("send success, hash = {}", hash);
    // sign_and_push(&rpc_client, tx).await;
}

#[tokio::test]
async fn request_project_request_cell() {
    let cell_deps = vec![
        KoCellDep::new(SECP256K1_TX_HASH.clone(), 0, DepType::DepGroup.into()),
        KoCellDep::new(KNSIDEOUT_TX_HASH.clone(), 0, DepType::Code.into()),
    ];

    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let mut backend = BackendImpl::new(&rpc_client);
    let mut previous_cell = None;
    let function_call = "battle_win()".into();
    if function_call == "claim_nfts" {
        // search previous personal cell
        let personal_data = backend
            .search_personal_data(OWNER_ADDRESS.into(), &PROJECT_CODE_HASH, &PROJECT_TYPE_ARGS)
            .await
            .expect("search personal");
        previous_cell = {
            if let Some((_, outpoint)) = personal_data.first() {
                Some(outpoint.clone())
            } else {
                None
            }
        };
    }
    println!("previous = {:?}", previous_cell);
    // create digest
    let digest = backend
        .create_project_request_digest(
            OWNER_ADDRESS.into(),
            None,
            previous_cell,
            function_call,
            &PROJECT_CODE_HASH,
            &PROJECT_TYPE_ARGS,
            &cell_deps,
        )
        .await
        .expect("create digest");

    // sign and push transaction
    let tx = backend.peak_transaction(&digest).expect("peak");
    let signature = sign(&rpc_client, tx);
    let hash = backend
        .send_transaction_to_ckb(&digest, &signature)
        .await
        .expect("send")
        .unwrap();
    println!("send success, hash = {}", hash);
    // sign_and_push(&rpc_client, tx).await;
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
