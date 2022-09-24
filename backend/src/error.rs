use ko_protocol::derive_more::Display;
use ko_protocol::types::error::{ErrorType, KoError};
use ko_protocol::H256;

#[derive(Display, Debug)]
pub enum BackendError {
    #[display(fmt = "Invalid contract byte code to deploy, error = {}", _0)]
    BadContractByteCode(String),

    #[display(fmt = "Throw error while calling `construct()`, error = {}", _0)]
    ConstructFunctionError(String),

    #[display(fmt = "Cannot create KOC global table, error = {}", _0)]
    CreateKOCTableError(String),

    #[display(fmt = "Cannot inject KOC global context, error = {}", _0)]
    InjectKOCContextError(String),

    #[display(fmt = "Throw error while jsonify global table, error = {}", _0)]
    GlobalTableNotJsonify(String),

    #[display(fmt = "Bad contract constructor return type, error = {}", _0)]
    InvalidConstructReturnType(String),

    #[display(fmt = "Bad driver filled in contract constructor")]
    InvalidSpecificContractDriver,

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

    #[display(fmt = "Rpc send_transaction error: {}, tx = {}", _0, _1)]
    TransactionSendError(String, String),

    #[display(fmt = "Lack of capacity: {} < {}", _0, _1)]
    InsufficientCapacity(u64, u64),

    #[display(fmt = "Request hash not found: {}", _0)]
    InvalidRequestHash(H256),

    #[display(fmt = "Cannot find managed global cell, type_args = {}", _0)]
    MissManagedGlobalCell(H256),

    #[display(fmt = "Porject is already managed, type_args = {}", _0)]
    AlreadyManagedProject(H256),
}

impl std::error::Error for BackendError {}

impl From<BackendError> for KoError {
    fn from(error: BackendError) -> KoError {
        KoError::new(ErrorType::Deployer, Box::new(error))
    }
}
