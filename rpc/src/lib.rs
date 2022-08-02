use ko_protocol::{traits::CkbClient, KoResult, ProjectDeps};
use ko_rpc_backend::BackendImpl;
use ko_rpc_server::RpcServer;

#[cfg(test)]
mod tests;

pub struct RpcServerRuntime {}

impl RpcServerRuntime {
    pub async fn run<C: CkbClient + 'static>(
        endpoint: &str,
        rpc_client: &C,
        project_deps: &ProjectDeps,
    ) -> KoResult<()> {
        let backend = BackendImpl::new(rpc_client);
        let handle = RpcServer::start(endpoint, backend, project_deps).await?;
        Box::leak(Box::new(handle));
        println!("[INFO] rpc server running at {}", endpoint);
        Ok(())
    }
}
