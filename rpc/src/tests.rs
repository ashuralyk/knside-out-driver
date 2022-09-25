use std::str::FromStr;

use ko_backend::BackendImpl;
use ko_protocol::ckb_jsonrpc_types::OutPoint;
use ko_protocol::secp256k1::{Message, SecretKey};
use ko_protocol::types::server::*;
use ko_protocol::{ckb_sdk::SECP256K1, ckb_types::H256, hex, tokio, TestVars::*};
use ko_rpc_client::RpcClient;
use ko_rpc_server::RpcServer;

use jsonrpsee::core::{client::ClientT, rpc_params};
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};

const JSONRPC_PORT: &str = "127.0.0.1:8090";

async fn create_server_and_client(with_server: bool) -> HttpClient {
    // start rpc server
    if with_server {
        let rpc_client = RpcClient::new(CKB_URL, CKB_INDEXER_URL);
        let backend = BackendImpl::new(&rpc_client, MockContextRpc::default());
        let handle = RpcServer::<_>::start(JSONRPC_PORT, backend, &PROJECT_VARS)
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
    let response: Result<KoMakeRequestTransactionDigestResponse, _> =
        create_server_and_client(false)
            .await
            .request(
                "ko_makeRequestTransactionDigest",
                rpc_params!(
                    String::from(OWNER_ADDRESS),
                    String::from("battle_win()"),
                    Option::<String>::None,
                    Option::<OutPoint>::None,
                    PROJECT_TYPE_ARGS
                ),
            )
            .await;
    println!("response = {:?}", response);
}

#[tokio::test]
async fn call_contract_method() {
    let client = create_server_and_client(false).await;

    // make digest
    let digest = {
        let response: KoMakeRequestTransactionDigestResponse = client
            .request(
                "ko_makeRequestTransactionDigest",
                rpc_params!(
                    String::from(OWNER_ADDRESS),
                    String::from("withdraw(100)"),
                    Option::<String>::None,
                    Option::<OutPoint>::None,
                    PROJECT_TYPE_ARGS
                ),
            )
            .await
            .expect("server response");
        println!("payment_ckb = {}", response.payment);
        H256::from_str(response.digest.as_str()).expect("digest")
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
    let hash: String = client
        .request(
            "ko_sendTransactionSignature",
            rpc_params!(digest, signature),
        )
        .await
        .expect("server response");
    println!("hash = {}", hash);

    // wait committed
    let hash = H256::from_str(hash.trim_start_matches("0x")).unwrap();
    let committed_hash: Option<H256> = client
        .request(
            "ko_waitRequestTransactionCommitted",
            rpc_params!(hash, PROJECT_TYPE_ARGS),
        )
        .await
        .expect("server commit");
    println!("committed = {}", hex::encode(&committed_hash.unwrap()));
}

#[tokio::test]
async fn send_fetch_global_data() {
    // send client request
    let response: String = create_server_and_client(false)
        .await
        .request("ko_fetchGlobalData", rpc_params!(PROJECT_TYPE_ARGS))
        .await
        .expect("server response");
    println!("response = {:?}", response);
}

#[tokio::test]
async fn send_fetch_personal_data() {
    // send client request
    let response: KoFetchPersonalDataResponse = create_server_and_client(false)
        .await
        .request(
            "ko_fetchPersonalData",
            rpc_params!(OWNER_ADDRESS, PROJECT_TYPE_ARGS),
        )
        .await
        .expect("server response");
    println!("response = {:?}", response);
}
