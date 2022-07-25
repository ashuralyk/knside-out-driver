use derive_more::Constructor;
use ckb_types::bytes::Bytes;

#[derive(Constructor)]
pub struct KoExecuteReceipt {
    pub outputs_json_data: Vec<Bytes>,
    pub required_payments: Vec<u64>
}
