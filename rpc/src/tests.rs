use ko_protocol::types::server::*;
use ko_protocol::{tokio, TestVars::*};
use ko_rpc_backend::BackendImpl;
use ko_rpc_client::RpcClient;
use ko_rpc_server::RpcServer;

use jsonrpsee::core::{client::ClientT, rpc_params};
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};

const JSONRPC_PORT: &str = "127.0.0.1:8090";

async fn create_server_and_client(with_server: bool) -> HttpClient {
    // start rpc server
    if with_server {
        let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
        let backend = BackendImpl::new(&rpc_client);
        let handle = RpcServer::<BackendImpl<RpcClient>>::start(JSONRPC_PORT, backend, &PROJECT_VARS)
            .await
            .expect("start rpc server");
        Box::leak(Box::new(handle));
    }
    HttpClientBuilder::default()
        .build("http://127.0.0.1:8090")
        .expect("start client")
}

#[tokio::test]
async fn send_make_request_digest() {
    let client = create_server_and_client(false).await;

    // send client request
    let params = KoMakeRequestDigestParams {
        sender: OWNER_ADDRESS.into(),
        payment: "50".into(),
        contract_call: "battle_win()".into(),
        private_key: OWNER_PRIVATE_KEY,
        recipient: None,
        previous_cell: None,
    };
    let response: String = client
        .request("make_request_digest", rpc_params!(params))
        .await
        .expect("server response");
    println!("response = {:?}", response);
}

#[tokio::test]
async fn send_fetch_global_data() {
    let client = create_server_and_client(false).await;

    // send client request
    let response: String = client
        .request("fetch_global_data", None)
        .await
        .expect("server response");
    println!("response = {:?}", response);
}

#[tokio::test]
async fn send_fetch_personal_data() {
    let client = create_server_and_client(false).await;

    // send client request
    let response: KoFetchPersonalDataResponse = client
        .request("fetch_personal_data", rpc_params!(OWNER_ADDRESS))
        .await
        .expect("server response");
    println!("response = {:?}", response);
}
