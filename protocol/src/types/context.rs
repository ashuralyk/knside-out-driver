use ckb_types::{packed::Script, H256};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub enum KoContextRpcEcho {
    EstimatePaymentCkb(
        (
            (Script, String, String, Option<Script>),
            UnboundedSender<u64>,
        ),
    ),
    ListenRequestCommitted((H256, UnboundedSender<H256>)),
}
