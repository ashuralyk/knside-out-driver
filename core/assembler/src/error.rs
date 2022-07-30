use ko_protocol::ckb_types::H256;
use ko_protocol::derive_more::Display;
use ko_protocol::types::error::{ErrorType, KoError};

#[derive(Display, Debug)]
pub enum AssemblerError {
    #[display(fmt = "{}", _0)]
    IndexerRpcError(String),

    #[display(fmt = "Project cell not found, project_id_args = {}", _0)]
    MissProjectDeploymentCell(H256),

    #[display(fmt = "Global cell not found, project_id = {}", _0)]
    MissProjectGlobalCell(H256),

    #[display(fmt = "Request cell not found")]
    MissProjectRequestCell,

    #[display(fmt = "Deployment format is not supported")]
    UnsupportedDeploymentFormat,

    #[display(fmt = "Caller lock_script format is not supported")]
    UnsupportedCallerScriptFormat,

    #[display(fmt = "Recipient lock_script format is not supported")]
    UnsupportedRecipientScriptFormat,

    #[display(fmt = "Global cell needs more {} ckb", _0)]
    InsufficientGlobalCellCapacity(u64),

    #[display(fmt = "Transaction capacity mismatch ({}:{})", _0, _1)]
    TransactionCapacityError(u64, u64),
}

impl std::error::Error for AssemblerError {}

impl From<AssemblerError> for KoError {
    fn from(error: AssemblerError) -> KoError {
        KoError::new(ErrorType::Assembler, Box::new(error))
    }
}
