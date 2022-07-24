use crate::ckb_types::core::TransactionView;
use crate::ckb_types::bytes::Bytes;
use crate::ckb_types::H256;
use crate::secp256k1::SecretKey;
use crate::{KoResult, async_trait};

#[async_trait]
pub trait Driver {
    async fn fetch_transactions_from_blocks_range(
        &self,
        begin_blocknumber: u64,
        end_blocknumber: u64
    ) -> KoResult<Vec<TransactionView>>;

    fn sign_ko_transaction(&self, tx: &TransactionView, privkey: &SecretKey) -> Bytes;

    fn send_ko_transaction(&mut self, tx: TransactionView) -> KoResult<H256>;
}
