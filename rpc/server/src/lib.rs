use std::net::SocketAddr;
use std::sync::Arc;

use jsonrpsee::http_server::{HttpServerBuilder, HttpServerHandle};
use jsonrpsee::{core::Error, proc_macros::rpc, types::error::CallError};
use ko_protocol::ckb_jsonrpc_types::OutPoint;
use ko_protocol::ckb_sdk::HumanCapacity;
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::tokio::sync::Mutex;
use ko_protocol::traits::Backend;
use ko_protocol::types::backend::KoRequestInput;
use ko_protocol::ProjectDeps;
use ko_protocol::{async_trait, hex, log, types::server::*, KoResult, H256};

mod error;
use error::RpcServerError;

type RpcResult<T> = Result<T, Error>;

#[rpc(server)]
trait KnsideRpc {
    #[method(name = "ko_version")]
    async fn version(&self) -> RpcResult<String>;

    #[method(name = "ko_makeDeployTransactionDigest")]
    async fn make_deploy_transaction_digest(
        &self,
        sender: String,
        contract_code: String,
    ) -> RpcResult<KoMakeDeployTransactionDigestResponse>;

    #[method(name = "ko_makeUpgradeTransactionDigest")]
    async fn make_upgrade_transaction_digest(
        &self,
        sender: String,
        new_contract_code: String,
        project_type_args: H256,
    ) -> RpcResult<H256>;

    #[method(name = "ko_makeRequestTransactionDigest")]
    async fn make_request_transaction_digest(
        &self,
        contract_call: String,
        inputs: Vec<OutPoint>,
        candidates: Vec<String>,
        components: Vec<OutPoint>,
        project_type_args: H256,
    ) -> RpcResult<KoMakeRequestTransactionDigestResponse>;

    #[method(name = "ko_sendTransactionSignature")]
    async fn send_transaction_signature(&self, digest: H256, signature: String) -> RpcResult<H256>;

    #[method(name = "ko_waitRequestTransactionCommitted")]
    async fn wait_request_transaction_committed(
        &self,
        request_hash: H256,
        project_type_args: H256,
    ) -> RpcResult<Option<H256>>;

    #[method(name = "ko_manageGlobalDataDriver")]
    async fn manage_global_data_driver(&self, project_type_args: H256) -> RpcResult<()>;

    #[method(name = "ko_fetchGlobalData")]
    async fn fetch_global_data(&self, project_type_args: H256) -> RpcResult<String>;

    #[method(name = "ko_fetchPersonalData")]
    async fn fetch_personal_data(
        &self,
        address: String,
        project_type_args: H256,
    ) -> RpcResult<KoFetchPersonalDataResponse>;
}

pub struct RpcServer<B: Backend + 'static> {
    ctx: Arc<Context<B>>,
}

// define all of rpc methods here
#[async_trait]
impl<B: Backend + 'static> KnsideRpcServer for RpcServer<B> {
    async fn version(&self) -> RpcResult<String> {
        log::debug!("[RPC] receive `version` rpc call");
        Ok("1.0.0".into())
    }

    async fn make_deploy_transaction_digest(
        &self,
        sender: String,
        contract_code: String,
    ) -> RpcResult<KoMakeDeployTransactionDigestResponse> {
        log::debug!(
            "[RPC] receive `make_deploy_transaction_digest` rpc call <= {}",
            sender,
        );
        let contract = hex::decode(contract_code).map_err(|err| Error::Custom(err.to_string()))?;
        let mut backend = self.ctx.backend.lock().await;
        let (digest, project_type_args) = backend
            .create_project_deploy_digest(Bytes::from(contract), sender, &self.ctx.project_deps)
            .await
            .map_err(|err| Error::Custom(err.to_string()))?;
        let result = KoMakeDeployTransactionDigestResponse::new(
            hex::encode(digest),
            hex::encode(project_type_args),
        );
        Ok(result)
    }

    async fn make_upgrade_transaction_digest(
        &self,
        sender: String,
        new_contract_code: String,
        project_type_args: H256,
    ) -> RpcResult<H256> {
        log::debug!(
            "[RPC] receive `make_upgrade_transaction_digest` rpc call <= {}({})",
            sender,
            project_type_args
        );
        let contract =
            hex::decode(new_contract_code).map_err(|err| Error::Custom(err.to_string()))?;
        let mut backend = self.ctx.backend.lock().await;
        let digest = backend
            .create_project_upgrade_digest(
                Bytes::from(contract),
                sender,
                &project_type_args,
                &self.ctx.project_deps,
            )
            .await
            .map_err(|err| Error::Custom(err.to_string()))?;
        Ok(digest)
    }

    async fn make_request_transaction_digest(
        &self,
        contract_call: String,
        inputs: Vec<OutPoint>,
        candidates: Vec<String>,
        components: Vec<OutPoint>,
        project_type_args: H256,
    ) -> RpcResult<KoMakeRequestTransactionDigestResponse> {
        log::debug!(
            "[RPC] receive `make_request_transaction_digest` rpc call <= {}",
            contract_call
        );
        let mut backend = self.ctx.backend.lock().await;
        let inputs = inputs.iter().map(|v| v.clone().into()).collect::<Vec<_>>();
        let components = components
            .iter()
            .map(|v| v.clone().into())
            .collect::<Vec<_>>();
        let (digest, payment_ckb) = backend
            .create_project_request_digest(
                contract_call,
                KoRequestInput::Outpoints(inputs),
                components.as_slice(),
                candidates.as_slice(),
                &project_type_args,
                &self.ctx.project_deps,
            )
            .await
            .map_err(|err| Error::Custom(err.to_string()))?;
        let result = KoMakeRequestTransactionDigestResponse::new(
            hex::encode(digest),
            HumanCapacity::from(payment_ckb).to_string(),
        );
        Ok(result)
    }

    async fn send_transaction_signature(&self, digest: H256, signature: String) -> RpcResult<H256> {
        log::debug!(
            "[RPC] receive `send_transaction_signature` rpc call <= digest({})",
            digest
        );
        let signature = hex::decode(signature).map_err(|_| {
            Error::Call(CallError::InvalidParams(
                RpcServerError::InvalidSignatureHexBytes.into(),
            ))
        })?;
        if signature.len() != 65 {
            return Err(Error::Call(CallError::InvalidParams(
                RpcServerError::InvalidSignatureLength(signature.len()).into(),
            )));
        }
        let mut signature_bytes = [0u8; 65];
        signature_bytes.copy_from_slice(&signature);

        self.ctx
            .backend
            .lock()
            .await
            .send_transaction_to_ckb(&digest, &signature_bytes)
            .await
            .map_err(|err| Error::Custom(err.to_string()))?
            .ok_or_else(|| Error::Custom(RpcServerError::SendSignature.to_string()))
    }

    async fn wait_request_transaction_committed(
        &self,
        request_hash: H256,
        project_type_args: H256,
    ) -> RpcResult<Option<H256>> {
        log::debug!(
            "[RPC] receive `wait_request_transaction_committed` rpc call <= hash({})",
            hex::encode(&request_hash)
        );
        self.ctx
            .backend
            .lock()
            .await
            .check_project_request_committed(
                &request_hash,
                &project_type_args,
                &self.ctx.project_deps,
            )
            .await
            .map_err(|err| Error::Custom(err.to_string()))
    }

    async fn manage_global_data_driver(&self, project_type_args: H256) -> RpcResult<()> {
        log::debug!(
            "[RPC] receive `manage_global_drive` rpc call => {}",
            hex::encode(&project_type_args)
        );
        self.ctx
            .backend
            .lock()
            .await
            .drive_project_on_management(&project_type_args, &self.ctx.project_deps)
            .await
            .map_err(|err| Error::Custom(err.to_string()))?;
        Ok(())
    }

    async fn fetch_global_data(&self, project_type_args: H256) -> RpcResult<String> {
        let global_data = self
            .ctx
            .backend
            .lock()
            .await
            .search_global_data(&project_type_args, &self.ctx.project_deps)
            .await
            .map_err(|err| Error::Custom(err.to_string()));
        log::debug!(
            "[RPC] receive `fetch_global_data` rpc call => {:?}",
            global_data
        );
        global_data
    }

    async fn fetch_personal_data(
        &self,
        address: String,
        project_type_args: H256,
    ) -> RpcResult<KoFetchPersonalDataResponse> {
        log::debug!("[RPC] receive `fetch_global_data` rpc call <= {}", address);
        let personal_data = self
            .ctx
            .backend
            .lock()
            .await
            .search_personal_data(address, &project_type_args, &self.ctx.project_deps)
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
