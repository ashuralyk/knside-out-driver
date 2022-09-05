use ckb_types::packed::Script;
use tokio::sync::mpsc::UnboundedSender;

use crate::{KoResult, H256};

#[derive(Debug)]
pub enum KoContextRpcEcho {
    #[allow(clippy::type_complexity)]
    EstimatePaymentCkb(
        (
            (Script, String, String, Option<Script>),
            UnboundedSender<KoResult<u64>>,
        ),
    ),
    ListenRequestCommitted((H256, UnboundedSender<KoResult<H256>>)),
}
