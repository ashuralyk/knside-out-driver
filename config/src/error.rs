use ko_protocol::derive_more::Display;
use ko_protocol::types::error::{ErrorType, KoError};

#[derive(Display, Debug)]
pub enum ConfigError {
    #[display(fmt = "Invalid config path: {}, reason = {}", _0, _1)]
    LoadingConfig(String, String),

    #[display(fmt = "Invalid project_type_args config path: {}, reason = {}", _0, _1)]
    LoadingConfigTypeArgs(String, String),

    #[display(fmt = "Invalid project_type_args config value, reason = {}", _0)]
    SavingConfigTypeArgs(String),
}

impl std::error::Error for ConfigError {}

impl From<ConfigError> for KoError {
    fn from(error: ConfigError) -> KoError {
        KoError::new(ErrorType::Config, Box::new(error))
    }
}
