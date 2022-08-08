use ckb_jsonrpc_types::OutPoint;
use ckb_types::H256;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{traits::Backend, ProjectDeps};

#[derive(Deserialize, Serialize)]
pub struct KoMakeRequestDigestParams {
    pub sender: String,
    pub payment: String,
    pub contract_call: String,
    pub recipient: Option<String>,
    pub previous_cell: Option<OutPoint>,
}

#[derive(Deserialize, Serialize)]
pub struct KoSendDigestSignatureParams {
    pub digest: H256,
    pub signature: String,
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
    pub project_deps: ProjectDeps,
    pub backend: Mutex<B>,
}

unsafe impl<B: Backend + 'static> Send for Context<B> {}
unsafe impl<B: Backend + 'static> Sync for Context<B> {}
