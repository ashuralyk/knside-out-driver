use ko_protocol::derive_more::Display;
use ko_protocol::types::error::{ErrorType, KoError};

#[derive(Display, Debug)]
pub enum AssemblerError {
    #[display(fmt = "Project cell not found, project_id_args = {:?}", _0)]
    MissProjectDeploymentCell([u8; 32]),

    #[display(fmt = "Global cell not found, project_id = {:?}", _0)]
    MissProjectGlobalCell([u8; 32]),

    #[display(fmt = "Deployment format is not supported")]
    UnsupportedDeploymentFormat,

    #[display(fmt = "Caller lock_script format is not supported")]
    UnsupportedCallerScriptFormat,

    #[display(
        fmt = "Length of outputs data and inputs lock_scripts are mismatched ({}:{})",
        _0,
        _1
    )]
    ScriptsAndOutputsDataMismatch(usize, usize),

    #[display(
        fmt = "Transaction inputs and outputs capacity are mismatched ({}:{})",
        _0,
        _1
    )]
    TransactionCapacityError(u64, u64),
}

impl std::error::Error for AssemblerError {}

impl From<AssemblerError> for KoError {
    fn from(error: AssemblerError) -> KoError {
        KoError::new(ErrorType::Assembler, Box::new(error))
    }
}
