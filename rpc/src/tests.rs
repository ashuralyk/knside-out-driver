use std::str::FromStr;

use ko_protocol::secp256k1::{Message, SecretKey};
use ko_protocol::types::server::*;
use ko_protocol::{ckb_sdk::SECP256K1, ckb_types::H256, hex, tokio, TestVars::*};
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
        let handle =
            RpcServer::<BackendImpl<RpcClient>>::start(JSONRPC_PORT, backend, &PROJECT_VARS)
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
    // send client request
    let params = KoMakeRequestDigestParams {
        sender: OWNER_ADDRESS.into(),
        payment: "50".into(),
        contract_call: "battle_win()".into(),
        recipient: None,
        previous_cell: None,
    };
    let response: String = create_server_and_client(false)
        .await
        .request("make_request_digest", rpc_params!(params))
        .await
        .expect("server response");
    println!("response = {:?}", response);
}

#[tokio::test]
async fn call_contract_method() {
    let client = create_server_and_client(false).await;

    // make digest
    let params = KoMakeRequestDigestParams {
        sender: OWNER_ADDRESS.into(),
        payment: "0".into(),
        contract_call: "battle_win()".into(),
        recipient: None,
        previous_cell: None,
    };
    let digest = {
        let digest: String = client
            .request("make_request_digest", rpc_params!(params))
            .await
            .expect("server response");
        H256::from_str(format!("0x{}", digest).as_str()).expect("digest")
    };
    // sign transaction
    let privkey = SecretKey::from_slice(OWNER_PRIVATE_KEY.as_bytes()).expect("privkey");
    let message = Message::from_slice(digest.as_bytes()).expect("digest");
    let recoverable = SECP256K1.sign_recoverable(&message, &privkey);
    let signature_bytes = {
        let (recover_id, signature) = recoverable.serialize_compact();
        let mut bytes = signature.to_vec();
        bytes.push(recover_id.to_i32() as u8);
        let mut signature = [0u8; 65];
        signature.copy_from_slice(&bytes);
        signature
    };
    let signature = hex::encode(&signature_bytes);
    // send transaction
    let params = KoSendDigestSignatureParams { digest, signature };
    let hash: String = client
        .request("send_digest_signature", rpc_params!(params))
        .await
        .expect("server response");
    println!("response = {:?}", hash);
}

#[tokio::test]
async fn send_fetch_global_data() {
    // send client request
    let response: String = create_server_and_client(false)
        .await
        .request("fetch_global_data", None)
        .await
        .expect("server response");
    println!("response = {:?}", response);
}

#[tokio::test]
async fn send_fetch_personal_data() {
    // send client request
    let response: KoFetchPersonalDataResponse = create_server_and_client(false)
        .await
        .request("fetch_personal_data", rpc_params!(OWNER_ADDRESS))
        .await
        .expect("server response");
    println!("response = {:?}", response);
}
