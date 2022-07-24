use crate::derive_more::Constructor;

#[derive(Constructor)]
pub struct KoExecuteReceipt {
    pub outputs_json_data: Vec<String>,
    pub required_ckbs: Vec<u64>,
}
