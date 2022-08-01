use ko_protocol::ckb_types::core::DepType;
use ko_protocol::ckb_types::H256;
use ko_protocol::traits::MockBackend;
use ko_protocol::types::{config::KoCellDep, server::*};
use ko_protocol::{hex, tokio, TestVars::*};
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

    // send client request
    let client = HttpClientBuilder::default()
        .build("http://127.0.0.1:8090")
        .expect("start client");
    let params = KoMakeReqeustDigestParams {
        sender: OWNER_ADDRESS.into(),
        contract_call: "battle_win()".into(),
        recipient: None,
        previous_cell: None,
    };
    let response: KoMakeReqeustDigestResponse = client
        .request("make_request_digest", rpc_params!(params))
        .await
        .expect("server response");
    println!("response = {:?}", response);
}

#[tokio::test]
async fn send_digest_signature() {
    use ko_protocol::mockall::predicate::*;

    const DIGEST_WITH_TX: [u8; 32] = [2u8; 32];
    const SIGNATURE: [u8; 65] = [1u8; 65];
    const TX_HASH: [u8; 32] = [3u8; 32];

    // start rpc server
    let mut backend = MockBackend::new();
    backend
        .expect_send_transaction_to_ckb()
        .with(eq(H256::from(DIGEST_WITH_TX)), eq(SIGNATURE))
        .times(1)
        .returning(|_digest, _signature| Ok(Some(TX_HASH.into())));

    let cell_deps = vec![
        KoCellDep::new(SECP256K1_TX_HASH.clone(), 0, DepType::DepGroup.into()),
        KoCellDep::new(KNSIDEOUT_TX_HASH.clone(), 0, DepType::Code.into()),
    ];
    let _handle = RpcServer::new("127.0.0.1:8091")
        .await
        .expect("build rpc server")
        .start(backend, &PROJECT_CODE_HASH, &PROJECT_TYPE_ARGS, &cell_deps)
        .await
        .expect("start rpc server");

    // send client request
    let client = HttpClientBuilder::default()
        .build("http://127.0.0.1:8091")
        .expect("start client");

    // test digest tx not found
    let params = KoSendDigestSignatureParams {
        digest: hex::encode(DIGEST_WITH_TX),
        signature: hex::encode(SIGNATURE),
    };
    let response: KoSendDigestSignatureResponse = client
        .request("send_digest_signature", rpc_params!(params))
        .await
        .expect("server response");
    println!("response = {:?}", response);

    assert_eq!(response.hash, hex::encode(TX_HASH));
}
