use crate::ckb_types::core::TransactionView;
use crate::ckb_types::bytes::Bytes;
use crate::ckb_types::H256;
use crate::KoResult;

pub trait Driver {
    fn sign_ko_transaction(&self, tx: &TransactionView) -> Bytes;

    fn send_ko_transaction(&mut self, tx: TransactionView) -> KoResult<H256>;
}
