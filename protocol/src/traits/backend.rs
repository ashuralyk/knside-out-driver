use crate::{async_trait, KoResult, ProjectDeps};
use ckb_types::{bytes::Bytes, packed::OutPoint, H256};

#[async_trait]
pub trait Backend: Send + Sync {
    async fn create_project_deploy_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_deps: &ProjectDeps,
    ) -> KoResult<(H256, H256)>;

    async fn create_project_update_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_deps: &ProjectDeps,
    ) -> KoResult<H256>;

    #[allow(clippy::too_many_arguments)]
    async fn create_project_request_digest(
        &mut self,
        address: String,
        payment_ckb: u64,
        recipient: Option<String>,
        previous_cell: Option<OutPoint>,
        function_call: String,
        project_deps: &ProjectDeps,
    ) -> KoResult<H256>;

    async fn send_transaction_to_ckb(
        &mut self,
        digest: &H256,
        signature: &[u8; 65],
    ) -> KoResult<Option<H256>>;

    async fn sign_transaction(&self, digest: &H256, privkey: &[u8]) -> KoResult<[u8; 65]>;

    async fn search_global_data(&self, project_deps: &ProjectDeps) -> KoResult<String>;

    async fn search_personal_data(
        &self,
        address: String,
        project_deps: &ProjectDeps,
    ) -> KoResult<Vec<(String, OutPoint)>>;
}
