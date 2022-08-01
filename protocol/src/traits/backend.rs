use crate::types::config::KoCellDep;
use crate::{async_trait, KoResult};
use ckb_types::{bytes::Bytes, packed::OutPoint, H256};

#[async_trait]
pub trait Backend: Send + Sync {
    async fn create_project_deploy_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_code_hash: &H256,
        project_cell_deps: &[KoCellDep],
    ) -> KoResult<(H256, H256)>;

    async fn create_project_update_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_type_args: &H256,
        project_cell_deps: &[KoCellDep],
    ) -> KoResult<H256>;

    #[allow(clippy::too_many_arguments)]
    async fn create_project_request_digest(
        &mut self,
        address: String,
        recipient: Option<String>,
        previous_cell: Option<OutPoint>,
        function_call: String,
        project_code_hash: &H256,
        project_type_args: &H256,
        project_cell_deps: &[KoCellDep],
    ) -> KoResult<H256>;

    async fn send_transaction_to_ckb(
        &mut self,
        digest: &H256,
        signature: &[u8; 65],
    ) -> KoResult<Option<H256>>;

    async fn search_global_data(
        &self,
        project_code_hash: &H256,
        project_type_args: &H256,
    ) -> KoResult<String>;

    async fn search_personal_data(
        &self,
        address: String,
        project_code_hash: &H256,
        project_type_args: &H256,
    ) -> KoResult<Vec<(String, OutPoint)>>;
}
