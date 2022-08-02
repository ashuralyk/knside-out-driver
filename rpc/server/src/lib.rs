use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use jsonrpsee::http_server::{HttpServerBuilder, HttpServerHandle};
use jsonrpsee::{core::Error, proc_macros::rpc, types::error::CallError};
use ko_protocol::ckb_sdk::HumanCapacity;
use ko_protocol::ckb_types::H256;
use ko_protocol::tokio::sync::Mutex;
use ko_protocol::traits::Backend;
use ko_protocol::ProjectDeps;
use ko_protocol::{async_trait, hex, types::server::*, KoResult};

mod error;
use error::RpcServerError;

type RpcResult<T> = Result<T, Error>;

#[rpc(server)]
trait KnsideRpc {
    #[method(name = "make_request_digest")]
    async fn make_request_digest(&self, payload: KoMakeRequestDigestParams) -> RpcResult<String>;

    #[method(name = "send_digest_signature")]
    async fn send_digest_signature(&self, payload: KoSendDigestSignatureParams) -> RpcResult<H256>;

    #[method(name = "fetch_global_data")]
    async fn fetch_global_data(&self) -> RpcResult<String>;

    #[method(name = "fetch_personal_data")]
    async fn fetch_personal_data(&self, address: String) -> RpcResult<KoFetchPersonalDataResponse>;
}

pub struct RpcServer<B: Backend + 'static> {
    ctx: Arc<Context<B>>,
}

// define all of rpc methods here
#[async_trait]
impl<B: Backend + 'static> KnsideRpcServer for RpcServer<B> {
    async fn make_request_digest(&self, payload: KoMakeRequestDigestParams) -> RpcResult<String> {
        println!(
            " [RPC] receive `make_request_digest` rpc call <= {}({})",
            payload.sender,
            payload.contract_call
        );
        let payment = HumanCapacity::from_str(&payload.payment).map_err(Error::Custom)?;
        let mut backend = self.ctx.backend.lock().await;
        let digest = backend
            .create_project_request_digest(
                payload.sender,
                payment.0,
                payload.recipient,
                payload.previous_cell.map(|v| v.into()),
                payload.contract_call,
                &self.ctx.project_deps,
            )
            .await
            .map_err(|err| Error::Custom(err.to_string()))?;
        let signature = backend
            .sign_transaction(&digest, payload.private_key.as_bytes())
            .await
            .map_err(|err| Error::Custom(err.to_string()))?;
        let hash = backend
            .send_transaction_to_ckb(&digest, &signature)
            .await
            .map_err(|err| Error::Custom(err.to_string()))?;
        Ok(hex::encode(hash.unwrap()))
    }

    async fn send_digest_signature(&self, payload: KoSendDigestSignatureParams) -> RpcResult<H256> {
        let mut sig = [0u8; 65];
        let payload_signature = payload.signature.as_bytes();

        if payload_signature.len() != 65 {
            return Err(Error::Call(CallError::InvalidParams(
                RpcServerError::InvalidSignatureLen(payload_signature.len()).into(),
            )));
        }

        sig.copy_from_slice(payload_signature);

        self.ctx
            .backend
            .lock()
            .await
            .send_transaction_to_ckb(&payload.digest, &sig)
            .await
            .map_err(|err| Error::Custom(err.to_string()))?
            .ok_or_else(|| Error::Custom(RpcServerError::SendSignature.to_string()))
    }

    async fn fetch_global_data(&self) -> RpcResult<String> {
        println!(" [RPC] receive `fetch_global_data` rpc call");
        self.ctx
            .backend
            .lock()
            .await
            .search_global_data(&self.ctx.project_deps)
            .await
            .map_err(|err| Error::Custom(err.to_string()))
    }

    async fn fetch_personal_data(&self, address: String) -> RpcResult<KoFetchPersonalDataResponse> {
        println!(" [RPC] receive `fetch_global_data` rpc call <= {}", address);
        let personal_data = self
            .ctx
            .backend
            .lock()
            .await
            .search_personal_data(address, &self.ctx.project_deps)
            .await
            .map_err(|err| Error::Custom(err.to_string()))?
            .into_iter()
            .map(|(data, outpoint)| KoPersonalData::new(data, outpoint.into()))
            .collect();
        Ok(KoFetchPersonalDataResponse::new(personal_data))
    }
}

impl<B: Backend + 'static> RpcServer<B> {
    pub async fn start(
        url: &str,
        backend: B,
        project_deps: &ProjectDeps,
    ) -> KoResult<HttpServerHandle> {
        let context = Context::new(project_deps.clone(), Mutex::new(backend));
        let rpc_impl = RpcServer {
            ctx: Arc::new(context),
        };

        // start jsonrpc server
        let server = HttpServerBuilder::default()
            .build(url.parse::<SocketAddr>().unwrap())
            .await
            .map_err(|err| RpcServerError::ErrorBuildRpcServer(err.to_string()))?;
        let handle = server
            .start(rpc_impl.into_rpc())
            .map_err(|err| RpcServerError::ErrorStartRpcServer(err.to_string()))?;
        Ok(handle)
    }
}
