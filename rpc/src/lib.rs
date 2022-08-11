use ko_protocol::tokio::sync::mpsc::UnboundedSender;
use ko_protocol::traits::CkbClient;
use ko_protocol::types::context::KoContextRpcEcho;
use ko_protocol::{KoResult, ProjectDeps};
use ko_rpc_backend::BackendImpl;
use ko_rpc_server::RpcServer;

#[cfg(test)]
mod tests;

pub struct RpcServerRuntime {}

impl RpcServerRuntime {
    pub async fn run<C: CkbClient + 'static>(
        endpoint: &str,
        rpc_client: &C,
        context_sender: UnboundedSender<KoContextRpcEcho>,
        project_deps: &ProjectDeps,
    ) -> KoResult<()> {
        let backend = BackendImpl::new(rpc_client, Some(context_sender));
        let handle = RpcServer::start(endpoint, backend, project_deps).await?;
        Box::leak(Box::new(handle));
        println!("[INFO] rpc server running at {}", endpoint);
        Ok(())
    }
}
