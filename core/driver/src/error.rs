use ko_protocol::ckb_types::H256;
use ko_protocol::derive_more::Display;
use ko_protocol::types::error::{ErrorType, KoError};

#[derive(Display, Debug)]
pub enum DriverError {
    #[display(fmt = "Rpc get_transaction error: {}, hash = {}", _0, _1)]
    ErrorFetchingCelldepTransaction(String, H256),

    #[display(fmt = "The block number is invalid, value = {}", _0)]
    InvalidBlockNumber(u64),

    #[display(fmt = "Rpc send_transaction error: {}, tx = {}", _0, _1)]
    TransactionSendError(String, String),

    #[display(fmt = "Rpc get_transaction error: {}, hash = {}", _0, _1)]
    TransactionFetchError(String, H256),
}

impl std::error::Error for DriverError {}

impl From<DriverError> for KoError {
    fn from(error: DriverError) -> KoError {
        KoError::new(ErrorType::Driver, Box::new(error))
    }
}
