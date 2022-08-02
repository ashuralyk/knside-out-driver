mod error;

use std::net::SocketAddr;
use std::sync::Arc;

use jsonrpsee::http_server::{HttpServerBuilder, HttpServerHandle};
use jsonrpsee::{core::Error, proc_macros::rpc, types::error::CallError};
use ko_protocol::ckb_types::H256;
use ko_protocol::tokio::sync::Mutex;
use ko_protocol::traits::Backend;
use ko_protocol::types::config::KoCellDep;
use ko_protocol::{hex, types::server::*, KoResult, async_trait};

use crate::error::RpcServerError;

type RpcResult<T> = Result<T, Error>;

#[rpc(server)]
pub trait KnsideRpc {
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
        let digest = self
            .ctx
            .backend
            .lock()
            .await
            .create_project_request_digest(
                payload.sender,
                payload.recipient,
                payload.previous_cell.map(|v| v.into()),
                payload.contract_call,
                &self.ctx.project_code_hash,
                &self.ctx.project_type_args,
                &self.ctx.project_cell_deps,
            )
            .await
            .map_err(|err| Error::Custom(err.to_string()))?;

        Ok(hex::encode(digest))
    }

    async fn send_digest_signature(&self, payload: KoSendDigestSignatureParams) -> RpcResult<H256> {
        let mut sig = [0u8; 65];
        let payload_signature = payload.signature.as_bytes();

        if payload_signature.len() != 65 {
            return Err(Error::Call(CallError::InvalidParams(
                RpcServerError::InvalidSignatureLen(payload_signature.len()).into(),
            )));
        }

        sig.copy_from_slice(&payload_signature);

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
        Ok(hex::encode(
            self.ctx
                .backend
                .lock()
                .await
                .search_global_data(&self.ctx.project_code_hash, &self.ctx.project_type_args)
                .await
                .map_err(|err| Error::Custom(err.to_string()))?,
        ))
    }

    async fn fetch_personal_data(&self, address: String) -> RpcResult<KoFetchPersonalDataResponse> {
        Ok(KoFetchPersonalDataResponse::new(
            self.ctx
                .backend
                .lock()
                .await
                .search_personal_data(
                    address,
                    &self.ctx.project_code_hash,
                    &self.ctx.project_type_args,
                )
                .await
                .map_err(|err| Error::Custom(err.to_string()))?
                .into_iter()
                .map(|item| KoPersonalData::new(Hex::encode(&item.0), item.1.into()))
                .collect(),
        ))
    }
}

impl<B: Backend + 'static> RpcServer<B> {
    pub async fn start(
        url: &str,
        backend: impl Backend + 'static,
        project_code_hash: &H256,
        project_type_args: &H256,
        project_cell_deps: &[KoCellDep],
    ) -> KoResult<HttpServerHandle> {
        let context = Context::new(
            project_code_hash.clone(),
            project_type_args.clone(),
            project_cell_deps.to_owned(),
            Mutex::new(backend),
        );
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
