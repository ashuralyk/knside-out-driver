use ckb_types::{H256, bytes::Bytes, core::TransactionView, packed::OutPoint};
use crate::types::config::KoCellDep;
use crate::{KoResult, async_trait};

#[async_trait]
pub trait Backend {
    async fn create_project_deploy_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_code_hash: &H256,
        project_cell_deps: &Vec<KoCellDep>
    ) -> KoResult<(H256, H256)>;

    async fn create_project_update_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_type_args: &H256,
        project_cell_deps: &Vec<KoCellDep>
    ) -> KoResult<H256>;

    async fn create_project_request_digest(
        &mut self,
        address: String,
        previous_cell: Option<OutPoint>,
        function_call: String,
        project_code_hash: &H256,
        project_type_args: &H256,
        project_cell_deps: &Vec<KoCellDep>
    ) -> KoResult<H256>;

    async fn pop_transaction(&mut self, digest: &H256) -> Option<TransactionView>;

    fn search_global_data(
        &self,
        project_code_hash: &H256,
        project_type_args: &H256
    ) -> KoResult<String>;

    fn search_personal_data(
        &self,
        address: String,
        project_code_hash: &H256,
        project_type_args: &H256
    ) -> KoResult<Vec<(String, OutPoint)>>;
}
