use ko_protocol::derive_more::Display;
use ko_protocol::types::error::{ErrorType, KoError};

#[derive(Display, Debug)]
pub enum ExecutorError {
    #[display(fmt = "Invalid project lua code, reason = {}", _0)]
    ErrorLoadProjectLuaCode(String),

    #[display(fmt = "Global json data string is not UTF-8 format")]
    InvalidUTF8FormatForGlobalData,

    #[display(fmt = "Personal json data string is not UTF-8 format")]
    InvalidUTF8FormatForPersonalData,

    #[display(fmt = "Invalid request lua code, code = {}, reason = {}", _0, _1)]
    ErrorLoadRequestLuaCode(String, String),

    #[display(
        fmt = "The input_ckb({}) is less than the cost_ckb({}) in cell({})",
        _0,
        _1,
        _2
    )]
    InsufficientRequiredCkb(u64, u64, usize),
}

impl std::error::Error for ExecutorError {}

impl From<ExecutorError> for KoError {
    fn from(error: ExecutorError) -> KoError {
        KoError::new(ErrorType::Executor, Box::new(error))
    }
}
