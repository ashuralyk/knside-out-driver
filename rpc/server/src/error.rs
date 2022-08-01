use ko_protocol::derive_more::Display;
use ko_protocol::types::error::{ErrorType, KoError};

#[derive(Display, Debug)]
pub enum RpcServerError {
    #[display(fmt = "Build rpc server failed, reason = {}", _0)]
    ErorrBuildRpcServer(String),

    #[display(fmt = "Register rpc method failed, reason = {}", _0)]
    ErrorRegisterRpcMethod(String),

    #[display(fmt = "Start rpc server failed, reason = {}", _0)]
    ErrorStartRpcServer(String),
}

impl std::error::Error for RpcServerError {}

impl From<RpcServerError> for KoError {
    fn from(error: RpcServerError) -> KoError {
        KoError::new(ErrorType::RpcServer, Box::new(error))
    }
}
