use ko_protocol::ckb_types::H256;
use ko_protocol::derive_more::Display;
use ko_protocol::types::error::{ErrorType, KoError};

#[derive(Display, Debug)]
pub enum BackendError {
    #[display(fmt = "Invalid contract byte code to deploy, error = {}", _0)]
    BadContractByteCode(String),

    #[display(fmt = "Throw error while calling `construct()`, error = {}", _0)]
    MissConstructFunction(String),

    #[display(fmt = "Throw error while jsonify global table, error = {}", _0)]
    GlobalTableNotJsonify(String),

    #[display(fmt = "Address format not supported, address = {}", _0)]
    InvalidAddressFormat(String),

    #[display(fmt = "Input and output CKB is mismatched")]
    InternalTransactionAssembleError,

    #[display(fmt = "Bad indexer rpc call, error = {}", _0)]
    IndexerRpcError(String),

    #[display(fmt = "Bad ckb rpc call, error = {}", _0)]
    CkbRpcError(String),

    #[display(fmt = "Previous cell not found")]
    InvalidPrevousCell,

    #[display(fmt = "Project deployment cell not found, type_args = {}", _0)]
    MissProjectDeploymentCell(H256),

    #[display(fmt = "Project global cell not found, type_args = {}", _0)]
    MissProjectGlobalCell(H256),

    #[display(fmt = "Global data is not UTF-8 format, type_args = {}", _0)]
    InvalidGlobalDataFormat(H256),

    #[display(fmt = "Personal data is not UTF-8 format, type_args = {}", _0)]
    InvalidPersonalDataFormat(H256),
}

impl std::error::Error for BackendError {}

impl From<BackendError> for KoError {
    fn from(error: BackendError) -> KoError {
        KoError::new(ErrorType::Deployer, Box::new(error))
    }
}
