use ckb_types::packed::Script;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub enum KoContextRpcEcho {
    EstimatePaymentCkb(
        (
            (Script, String, String, Option<Script>),
            UnboundedSender<u64>,
        ),
    ),
}
