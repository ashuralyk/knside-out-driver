use ckb_sdk::traits::LiveCell;
use ckb_types::{bytes::Bytes, core::Capacity, packed::Script, prelude::Unpack};
use derive_more::Constructor;
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

#[derive(Default, Constructor)]
pub struct KoContextGlobalCell {
    pub lock_script: Script,
    pub output_data: Bytes,
    pub capacity: u64,
    pub occupied_capacity: u64,
}

impl From<LiveCell> for KoContextGlobalCell {
    fn from(cell: LiveCell) -> Self {
        let occupied = cell
            .output
            .occupied_capacity(Capacity::bytes(cell.output_data.len()).unwrap())
            .unwrap()
            .as_u64();
        let capacity = cell.output.capacity().unpack();
        KoContextGlobalCell::new(cell.output.lock(), cell.output_data, capacity, occupied)
    }
}
