use ko_backend::BackendImpl;
use ko_protocol::traits::{CkbClient, ContextRpc};
use ko_protocol::{log, KoResult, ProjectDeps};
use ko_rpc_server::RpcServer;

#[cfg(test)]
mod tests;

pub struct RpcServerRuntime {}

impl RpcServerRuntime {
    pub async fn run<C: CkbClient + 'static, R: ContextRpc + 'static>(
        endpoint: &str,
        backend: BackendImpl<C, R>,
        project_deps: &ProjectDeps,
    ) -> KoResult<()> {
        let handle = RpcServer::start(endpoint, backend, project_deps).await?;
        Box::leak(Box::new(handle));
        log::info!("rpc server running at {}", endpoint);
        Ok(())
    }
}
