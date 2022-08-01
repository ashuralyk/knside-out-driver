use derive_more::{Constructor, Display};
use std::error::Error;

#[derive(Debug)]
pub enum ErrorType {
    Assembler,
    Driver,
    Executor,
    Config,
    Deployer,
    Requester,
    CkbClient,
    RpcServer,
    Hex,
}

#[derive(Debug, Constructor, Display)]
#[display(fmt = "Type: {:?}, Message: {:?}", error_type, message)]
pub struct KoError {
    error_type: ErrorType,
    message: Box<dyn Error + Send>,
}

impl Error for KoError {}

pub type KoResult<T> = Result<T, KoError>;
