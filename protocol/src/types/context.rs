use ckb_types::{packed::Script, H256};
use tokio::sync::mpsc::UnboundedSender;

use crate::KoResult;

#[derive(Debug)]
pub enum KoContextRpcEcho {
    EstimatePaymentCkb(
        (
            (Script, String, String, Option<Script>),
            UnboundedSender<KoResult<u64>>,
        ),
    ),
    ListenRequestCommitted((H256, UnboundedSender<KoResult<H256>>)),
}
