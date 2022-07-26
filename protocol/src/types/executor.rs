use ckb_types::bytes::Bytes;
use derive_more::Constructor;

#[derive(Constructor)]
pub struct KoExecuteReceipt {
    pub outputs_json_data: Vec<Bytes>,
    pub required_payments: Vec<u64>,
}
