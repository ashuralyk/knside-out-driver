use ckb_jsonrpc_types::OutPoint;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MakeRequestDigestPayload {
    pub sender: String,
    pub contract_call: String,
    pub recipient: Option<String>,
    pub previous_cell: Option<OutPoint>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SendDigestSignaturePayload {
	pub digest: String,
	pub signature: String,
}
