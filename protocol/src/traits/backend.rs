use crate::{async_trait, KoResult, ProjectDeps, H256};
use ckb_types::{bytes::Bytes, packed::OutPoint};

#[async_trait]
pub trait Backend: Send + Sync {
    async fn create_project_deploy_digest(
        &mut self,
        contract: Bytes,
        address: String,
        management: bool,
        project_deps: &ProjectDeps,
    ) -> KoResult<(H256, H256)>;

    async fn create_project_upgrade_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<H256>;

    async fn create_project_request_digest(
        &mut self,
        address: String,
        recipient: Option<String>,
        previous_cell: Option<OutPoint>,
        function_call: String,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<(H256, u64)>;

    async fn check_project_request_committed(
        &mut self,
        transaction_hash: &H256,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<Option<H256>>;

    async fn drive_project_on_management(
        &mut self,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<()>;

    async fn send_transaction_to_ckb(
        &mut self,
        digest: &H256,
        signature: &[u8; 65],
    ) -> KoResult<Option<H256>>;

    async fn search_global_data(
        &self,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<String>;

    async fn search_personal_data(
        &self,
        address: String,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<Vec<(String, OutPoint)>>;
}
