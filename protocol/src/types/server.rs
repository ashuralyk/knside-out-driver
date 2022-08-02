use ckb_jsonrpc_types::OutPoint;
use ckb_types::H256;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{traits::Backend, types::config::KoCellDep};

#[derive(Deserialize, Serialize)]
pub struct KoMakeReqeustDigestParams {
    pub sender: String,
    pub contract_call: String,
    pub private_key: String,
    pub recipient: Option<String>,
    pub previous_cell: Option<OutPoint>,
}

#[derive(Serialize, Deserialize, Constructor, Debug)]
pub struct KoMakeReqeustDigestResponse {
    pub digest: String,
}

#[derive(Deserialize, Serialize)]
pub struct KoSendDigestSignatureParams {
    pub digest: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Constructor, Debug)]
pub struct KoSendDigestSignatureResponse {
    pub hash: String,
}

#[derive(Deserialize, Serialize)]
pub struct KoFetchGlobalDataParams {}

#[derive(Serialize, Constructor, Debug)]
pub struct KoFetchGlobalDataResponse {
    pub data: String,
}

#[derive(Deserialize, Serialize)]
pub struct KoFetchPersonalDataParams {
    pub address: String,
}

#[derive(Serialize, Deserialize, Constructor, Debug)]
pub struct KoPersonalData {
    pub data: String,
    pub outpoint: OutPoint,
}

#[derive(Serialize, Deserialize, Constructor, Debug)]
pub struct KoFetchPersonalDataResponse {
    pub data: Vec<KoPersonalData>,
}

#[derive(Constructor)]
pub struct Context<B: Backend + 'static> {
    pub project_code_hash: H256,
    pub project_type_args: H256,
    pub project_cell_deps: Vec<KoCellDep>,
    pub backend: Mutex<B>,
}

unsafe impl<B: Backend + 'static> Send for Context<B> {}
unsafe impl<B: Backend + 'static> Sync for Context<B> {}
