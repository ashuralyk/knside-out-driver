use ckb_types::bytes::Bytes;
use ckb_types::packed::Script;
use derive_more::Constructor;

use crate::KoResult;

#[derive(Constructor)]
pub struct KoExecuteReceipt {
    pub global_json_data: Bytes,
    pub personal_outputs: Vec<KoResult<(Option<Bytes>, Script)>>,
}
