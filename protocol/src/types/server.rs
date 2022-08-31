use ckb_jsonrpc_types::OutPoint;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{traits::Backend, ProjectDeps};

#[derive(Deserialize, Serialize, Constructor, Debug)]
pub struct KoMakeDeployTransactionDigestResponse {
    pub digest: String,
    pub project_type_args: String,
}

#[derive(Deserialize, Serialize, Constructor, Debug)]
pub struct KoMakeRequestTransactionDigestResponse {
    pub digest: String,
    pub payment: String,
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
