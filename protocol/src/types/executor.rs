use ckb_types::bytes::Bytes;
use ckb_types::packed::Script;

use crate::KoResult;

pub type KoExecutedRequest = KoResult<(Option<Bytes>, Script)>;
