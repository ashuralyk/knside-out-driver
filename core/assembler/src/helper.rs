use ko_protocol::ckb_sdk::constants::TYPE_ID_CODE_HASH;
use ko_protocol::ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey};
use ko_protocol::ckb_sdk::traits::LiveCell;
use ko_protocol::ckb_types::packed::{CellOutput, Script};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack};
use ko_protocol::ckb_types::{bytes::Bytes, core::ScriptHashType, H256};
use ko_protocol::traits::CkbClient;
use ko_protocol::{is_mol_flag_2, mol_deployment_raw, mol_flag_0, mol_flag_1, KoResult};

use crate::error::AssemblerError;

pub async fn search_project_cell(
    rpc: &impl CkbClient,
    project_id_args: &H256,
) -> KoResult<LiveCell> {
    let project_typescript = Script::new_builder()
        .code_hash(TYPE_ID_CODE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(project_id_args.0.as_slice().pack())
        .build();
    let search_key = SearchKey {
        script: project_typescript.into(),
        script_type: ScriptType::Type,
        filter: None,
    };
    let result = rpc
        .fetch_live_cells(search_key, 1, None)
        .await
        .map_err(|err| AssemblerError::IndexerRpcError(err.to_string()))?;
    if let Some(cell) = result.objects.first() {
        Ok((cell.clone()).into())
    } else {
        Err(AssemblerError::MissProjectDeploymentCell(project_id_args.clone()).into())
    }
}

pub async fn search_global_cell(
    rpc: &impl CkbClient,
    code_hash: &H256,
    project_id: &H256,
) -> KoResult<LiveCell> {
    let global_typescript = Script::new_builder()
        .code_hash(code_hash.pack())
        .hash_type(ScriptHashType::Data1.into())
        .args(mol_flag_0(&project_id.0).as_slice().pack())
        .build();
    let search_key = SearchKey {
        script: global_typescript.into(),
        script_type: ScriptType::Type,
        filter: None,
    };
    let result = rpc
        .fetch_live_cells(search_key, 1, None)
        .await
        .map_err(|_| AssemblerError::MissProjectGlobalCell(project_id.clone()))?;
    if let Some(cell) = result.objects.first() {
        Ok((cell.clone()).into())
    } else {
        Err(AssemblerError::MissProjectGlobalCell(project_id.clone()).into())
    }
}

pub fn make_global_script(code_hash: &H256, project_id: &H256) -> Script {
    Script::new_builder()
        .code_hash(code_hash.pack())
        .hash_type(ScriptHashType::Data1.into())
        .args(mol_flag_0(&project_id.0).as_slice().pack())
        .build()
}

pub fn make_personal_script(code_hash: &H256, project_id: &H256) -> Script {
    Script::new_builder()
        .code_hash(code_hash.pack())
        .hash_type(ScriptHashType::Data1.into())
        .args(mol_flag_1(&project_id.0).as_slice().pack())
        .build()
}

pub fn check_valid_request(cell: &CellOutput, code_hash: &H256) -> bool {
    let lock = &cell.lock();
    if lock.code_hash().as_slice() != code_hash.as_bytes()
        || lock.hash_type() != ScriptHashType::Data1.into()
        || !is_mol_flag_2(&lock.args().raw_data().to_vec())
    {
        return false;
    }
    true
}

pub fn extract_project_lua_code(deployment_bytes: &Bytes) -> KoResult<Bytes> {
    if let Some(deployment) = mol_deployment_raw(deployment_bytes) {
        Ok(deployment.code().raw_data())
    } else {
        Err(AssemblerError::UnsupportedDeploymentFormat.into())
    }
}
