use ko_protocol::traits::Assembler;
use ko_protocol::H256;
use ko_protocol::{hex, secp256k1::SecretKey, tokio, TestVars::*};
use ko_rpc_client::RpcClient;

use crate::ContextImpl;

#[tokio::test]
async fn drive_one() {
    // prepare parts
    let rpc_client = RpcClient::new(&CKB_URL, &CKB_INDEXER_URL);
    let privkey = SecretKey::from_slice(OWNER_PRIVATE_KEY.as_bytes()).expect("private key");
    let (mut ctx, _) = ContextImpl::new(
        &rpc_client,
        &privkey,
        &PROJECT_TYPE_ARGS.into(),
        &PROJECT_VARS,
        &DRIVE_CONFIG,
    );

    // prepare to make instance of context
    let project_dep = ctx
        .assembler
        .prepare_transaction_project_celldep()
        .await
        .expect("project dep");

    // drive knside-out transaction
    let hash = ctx.drive(&project_dep).await.expect("drive");
    println!("hash = {}", hex::encode(&hash.unwrap_or(H256::default())));
}
