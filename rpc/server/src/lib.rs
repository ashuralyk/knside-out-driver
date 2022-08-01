use std::net::SocketAddr;
use std::sync::Arc;

use jsonrpsee_core::Error;
use jsonrpsee_http_server::{HttpServer, HttpServerBuilder, HttpServerHandle, RpcModule};
use jsonrpsee_types::Params;
use ko_protocol::ckb_types::H256;
use ko_protocol::tokio::sync::Mutex;
use ko_protocol::traits::Backend;
use ko_protocol::types::config::KoCellDep;
use ko_protocol::{hex, types::server::*, KoResult};

mod error;
use error::RpcServerError;

macro_rules! register_async {
    ($module:expr, $name:ident) => {
        $module
            .register_async_method(stringify!($name), RpcServerInternal::$name)
            .map_err(|err| RpcServerError::ErrorRegisterRpcMethod(err.to_string()))?;
    };
}

struct RpcServerInternal {}

// define all of rpc methods here
impl RpcServerInternal {
    pub async fn make_request_digest(
        params: Params<'static>,
        ctx: Arc<Context<impl Backend>>,
    ) -> Result<KoMakeReqeustDigestResponse, Error> {
        let request: KoMakeReqeustDigestParams = params.one()?;
        let digest = ctx
            .backend
            .lock()
            .await
            .create_project_request_digest(
                request.sender,
                request.recipient,
                request.previous_cell.map(|v| v.into()),
                request.contract_call,
                &ctx.project_code_hash,
                &ctx.project_type_args,
                &ctx.project_cell_deps,
            )
            .await
            .map_err(|err| Error::Custom(err.to_string()))?;
        Ok(KoMakeReqeustDigestResponse::new(hex::encode(digest)))
    }

    pub async fn send_digest_signature(
        params: Params<'static>,
        ctx: Arc<Context<impl Backend>>,
    ) -> Result<KoSendDigestSignatureResponse, Error> {
        let request: KoSendDigestSignatureParams = params.one()?;

        let digest = {
            let mut buf = [0u8; 32];
            hex::decode_to_slice(request.digest.trim_start_matches("0x"), &mut buf)
                .map_err(|err| Error::Custom(err.to_string()))?;
            H256::from(buf)
        };
        let signature = {
            let mut buf = [0u8; 65];
            hex::decode_to_slice(request.signature.trim_start_matches("0x"), &mut buf)
                .map_err(|err| Error::Custom(err.to_string()))?;
            buf
        };

        let maybe_ok = {
            let mut backend = ctx.backend.lock().await;
            backend.send_transaction_to_ckb(&digest, &signature).await
        };
        match maybe_ok {
            Ok(Some(tx_hash)) => Ok(KoSendDigestSignatureResponse::new(hex::encode(tx_hash))),
            Ok(None) => Err(Error::Custom("digest transaction not found".to_string())),
            Err(err) => Err(Error::Custom(err.to_string())),
        }
    }

    pub async fn fetch_global_data(
        params: Params<'static>,
        ctx: Arc<Context<impl Backend>>,
    ) -> Result<KoFetchGlobalDataResponse, Error> {
        let response = KoFetchGlobalDataResponse::new("".into());
        Ok(response)
    }

    pub async fn fetch_personal_data(
        params: Params<'static>,
        ctx: Arc<Context<impl Backend>>,
    ) -> Result<KoFetchPersonalDataResponse, Error> {
        let response = KoFetchPersonalDataResponse::new(vec![]);
        Ok(response)
    }
}

pub struct RpcServer {
    rpc_server: HttpServer,
}

impl RpcServer {
    pub async fn new(url: &str) -> KoResult<Self> {
        let server = HttpServerBuilder::default()
            .build(url.parse::<SocketAddr>().unwrap())
            .await
            .map_err(|err| RpcServerError::ErorrBuildRpcServer(err.to_string()))?;
        let server = RpcServer { rpc_server: server };
        Ok(server)
    }

    pub async fn start(
        self,
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
        let mut module = RpcModule::new(context);
        // register jsonrpc methods
        register_async!(module, make_request_digest);
        register_async!(module, send_digest_signature);
        register_async!(module, fetch_global_data);
        register_async!(module, fetch_personal_data);
        // start jsonrpc server
        let handle = self
            .rpc_server
            .start(module)
            .map_err(|err| RpcServerError::ErrorStartRpcServer(err.to_string()))?;
        Ok(handle)
    }
}
