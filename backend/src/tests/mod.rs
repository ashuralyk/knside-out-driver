mod mock_rpc;

use ko_context::ContextImpl;
use ko_protocol::ckb_jsonrpc_types::TransactionView as JsonTxView;
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::core::TransactionView;
use ko_protocol::secp256k1::SecretKey;
use ko_protocol::traits::{Backend, CkbClient, Driver};
use ko_protocol::{hex, serde_json, tokio, TestVars::*};
use ko_rpc_client::RpcClient;

use crate::BackendImpl;
use mock_rpc::MockContextrpc;

fn sign(ctx: &ContextImpl<impl CkbClient>, tx: TransactionView) -> [u8; 65] {
    println!(
        "tx = {}",
        serde_json::to_string_pretty(&JsonTxView::from(tx.clone())).unwrap()
    );
    // sign transaction
    let signature = {
        let mut bytes = [0u8; 65];
        let signature = ctx.driver.sign_transaction(&tx);
        bytes.copy_from_slice(&signature);
        bytes
    };
    signature
}

#[tokio::test]
async fn deploy_project_deployment_cell() {
    let lua_code = std::fs::read_to_string("./src/tests/21-point.lua").unwrap();

    // create digest
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let privkey = SecretKey::from_slice(OWNER_PRIVATE_KEY.as_bytes()).unwrap();
    let (context, _) = ContextImpl::new(
        &rpc_client,
        &privkey,
        &PROJECT_TYPE_ARGS.into(),
        &PROJECT_VARS,
        &DRIVE_CONFIG,
    );
    let mut backend = BackendImpl::new(&rpc_client, MockContextrpc::default());
    let (digest, type_args) = backend
        .create_project_deploy_digest(
            Bytes::from(hex::encode(lua_code).as_bytes().to_vec()),
            OWNER_ADDRESS.into(),
            &PROJECT_VARS,
        )
        .await
        .expect("create digest");
    println!("project_type_args = {}", type_args);

    // sign and push transaction
    let tx = backend.peak_transaction(&digest).expect("peak");
    let signature = sign(&context, tx);
    let hash = backend
        .send_transaction_to_ckb(&digest, &signature)
        .await
        .expect("send")
        .unwrap();
    println!("send success, hash = {}", hash);
}

#[tokio::test]
async fn update_project_deployment_cell() {
    let lua_code = std::fs::read_to_string("./src/tests/21-point.lua").unwrap();

    // create digest
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let privkey = SecretKey::from_slice(OWNER_PRIVATE_KEY.as_bytes()).unwrap();
    let (context, _) = ContextImpl::new(
        &rpc_client,
        &privkey,
        &PROJECT_TYPE_ARGS.into(),
        &PROJECT_VARS,
        &DRIVE_CONFIG,
    );
    let mut backend = BackendImpl::new(&rpc_client, MockContextrpc::default());
    let digest = backend
        .create_project_upgrade_digest(
            Bytes::from(hex::encode(lua_code).as_bytes().to_vec()),
            OWNER_ADDRESS.into(),
            &PROJECT_TYPE_ARGS.into(),
            &PROJECT_VARS,
        )
        .await
        .expect("create digest");

    // sign and push transaction
    let tx = backend.peak_transaction(&digest).expect("peak");
    let signature = sign(&context, tx);
    let hash = backend
        .send_transaction_to_ckb(&digest, &signature)
        .await
        .expect("send")
        .unwrap();
    println!("send success, hash = {}", hash);
}

#[tokio::test]
async fn request_project_request_cell() {
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let privkey = SecretKey::from_slice(OWNER_PRIVATE_KEY.as_bytes()).unwrap();
    let (context, _) = ContextImpl::new(
        &rpc_client,
        &privkey,
        &PROJECT_TYPE_ARGS.into(),
        &PROJECT_VARS,
        &DRIVE_CONFIG,
    );
    let mut backend = BackendImpl::new(&rpc_client, MockContextrpc::default());
    let mut previous_cell = None;
    let function_call = "battle_win()".into();
    if function_call == "claim_nfts" {
        // search previous personal cell
        let personal_data = backend
            .search_personal_data(
                OWNER_ADDRESS.into(),
                &PROJECT_TYPE_ARGS.into(),
                &PROJECT_VARS,
            )
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
    let (digest, _) = backend
        .create_project_request_digest(
            OWNER_ADDRESS.into(),
            None,
            previous_cell,
            function_call,
            &PROJECT_TYPE_ARGS.into(),
            &PROJECT_VARS,
        )
        .await
        .expect("create digest");

    // sign and push transaction
    let tx = backend.peak_transaction(&digest).expect("peak");
    let signature = sign(&context, tx);
    let hash = backend
        .send_transaction_to_ckb(&digest, &signature)
        .await
        .expect("send")
        .unwrap();
    println!("send request success, hash = {}", hash);

    // check request committed
    // let committed_hash = backend
    //     .check_project_request_committed(&hash, &PROJECT_VARS)
    //     .await
    //     .expect("check");
    // println!("reqeust committed, hash = {:?}", committed_hash);
}

#[tokio::test]
async fn fetch_global_json_data() {
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let global_data = BackendImpl::new(&rpc_client, MockContextrpc::default())
        .search_global_data(&PROJECT_TYPE_ARGS.into(), &PROJECT_VARS)
        .await
        .expect("search global");
    println!("global_data = {}", global_data);
}

#[tokio::test]
async fn fetch_personal_json_data() {
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let personal_data = BackendImpl::new(&rpc_client, MockContextrpc::default())
        .search_personal_data(
            OWNER_ADDRESS.into(),
            &PROJECT_TYPE_ARGS.into(),
            &PROJECT_VARS,
        )
        .await
        .expect("search personal");
    personal_data.into_iter().for_each(|(data, outpoint)| {
        println!("personal_data = {}, outpoint = {}", data, outpoint);
    });
}
