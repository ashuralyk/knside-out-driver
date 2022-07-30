use ckb_types::bytes::Bytes;
use ckb_types::packed::Script;
use derive_more::Constructor;

#[derive(Constructor)]
pub struct KoExecuteReceipt {
    pub global_json_data: Bytes,
    pub personal_outputs: Vec<(Option<Bytes>, Script)>,
}
