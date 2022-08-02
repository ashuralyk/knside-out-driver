use ko_protocol::ckb_types::core::DepType;
use ko_protocol::types::{config::KoCellDep, server::*};
use ko_protocol::{tokio, TestVars::*, hex};
use ko_rpc_backend::BackendImpl;
use ko_rpc_client::RpcClient;
use ko_rpc_server::RpcServer;

use jsonrpsee_core::{client::ClientT, rpc_params};
use jsonrpsee_http_client::HttpClientBuilder;

#[tokio::test]
async fn send_make_request_digest() {
    // start rpc server
    let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
    let backend = BackendImpl::new(&rpc_client);
    let cell_deps = vec![
        KoCellDep::new(SECP256K1_TX_HASH.clone(), 0, DepType::DepGroup.into()),
        KoCellDep::new(KNSIDEOUT_TX_HASH.clone(), 0, DepType::Code.into()),
    ];
    let _handle = RpcServer::new("127.0.0.1:8090")
        .await
        .expect("build rpc server")
        .start(backend, &PROJECT_CODE_HASH, &PROJECT_TYPE_ARGS, &cell_deps)
        .await
        .expect("start rpc server");

    // loop {
    //     use std::time::Duration;
    //     tokio::time::sleep(Duration::from_secs(3)).await;
    //     println!("next loop");
    // }

    // send client request
    let client = HttpClientBuilder::default()
        .build("http://127.0.0.1:8090")
        .expect("start client");
    let params = KoMakeReqeustDigestParams {
        sender: OWNER_ADDRESS.into(),
        contract_call: "battle_win()".into(),
        private_key: hex::encode(OWNER_PRIVATE_KEY.as_bytes()),
        recipient: None,
        previous_cell: None,
    };
    let response: KoMakeReqeustDigestResponse = client
        .request("make_request_digest", rpc_params!(params))
        .await
        .expect("server response");
    println!("response = {:?}", response);
}
