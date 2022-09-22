use ko_protocol::derive_more::Display;
use ko_protocol::types::error::{ErrorType, KoError};
use mlua::Error;

#[derive(Display, Debug)]
pub enum ExecutorError {
    #[display(fmt = "Invalid project lua code, reason = {}", _0)]
    ErrorLoadProjectLuaCode(String),

    #[display(fmt = "Global data is not UTF-8 format")]
    InvalidUTF8FormatForGlobalData,

    #[display(fmt = "Global data is not a JSON string, value = {}", _0)]
    InvalidJsonFormatForGlobalData(String),

    #[display(fmt = "Personal data is not UTF-8 format")]
    InvalidUTF8FormatForPersonalData,

    #[display(fmt = "Personal data is not a JSON string")]
    InvalidJsonFormatForPersonalData,

    #[display(fmt = "Invalid request lua code, code = {}, reason = {}", _0, _1)]
    ErrorLoadRequestLuaCode(String, String),

    #[display(fmt = "koc.owner mismatch, {} != {}", _0, _1)]
    OwnerLockhashMismatch(String, String),

    #[display(fmt = "koc.user mismatch")]
    UnexpectedUserLockhash,

    #[display(fmt = "koc.driver mismatch")]
    UnexpectedDriverLockhash,

    #[display(fmt = "Lua code execution error = {}", _0)]
    LuaVmError(String),
}

impl std::error::Error for ExecutorError {}

impl From<ExecutorError> for KoError {
    fn from(error: ExecutorError) -> KoError {
        KoError::new(ErrorType::Executor, Box::new(error))
    }
}

impl From<Error> for ExecutorError {
    fn from(error: Error) -> Self {
        ExecutorError::LuaVmError(error.to_string())
    }
}
