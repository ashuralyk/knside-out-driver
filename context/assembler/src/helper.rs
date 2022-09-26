use ko_protocol::ckb_sdk::constants::TYPE_ID_CODE_HASH;
use ko_protocol::ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey, SearchKeyFilter};
use ko_protocol::ckb_sdk::traits::LiveCell;
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::core::{Capacity, ScriptHashType, TransactionView};
use ko_protocol::ckb_types::packed::{CellInput, CellOutput, Script};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack, Unpack};
use ko_protocol::traits::CkbClient;
use ko_protocol::{is_mol_flag_2, mol_flag_0, mol_flag_1, KoResult, H256};

use crate::error::AssemblerError;

pub async fn search_project_cell(
    rpc: &impl CkbClient,
    project_id_args: &H256,
) -> KoResult<LiveCell> {
    let project_typescript = Script::new_builder()
        .code_hash(TYPE_ID_CODE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(project_id_args.as_bytes().pack())
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
    driver: Option<&Script>,
) -> KoResult<LiveCell> {
    let global_typescript = Script::new_builder()
        .code_hash(code_hash.pack())
        .hash_type(ScriptHashType::Data.into())
        .args(mol_flag_0(project_id.as_bytes32()).as_slice().pack())
        .build();
    let mut search_key = SearchKey {
        script: global_typescript.into(),
        script_type: ScriptType::Type,
        filter: None,
    };
    if let Some(driver) = driver {
        let filter = SearchKeyFilter {
            script: Some(driver.clone().into()),
            output_data_len_range: None,
            output_capacity_range: None,
            block_range: None,
        };
        search_key.filter = Some(filter);
    }
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
        .hash_type(ScriptHashType::Data.into())
        .args(mol_flag_0(project_id.as_bytes32()).as_slice().pack())
        .build()
}

pub fn make_personal_script(code_hash: &H256, project_id: &H256) -> Script {
    Script::new_builder()
        .code_hash(code_hash.pack())
        .hash_type(ScriptHashType::Data.into())
        .args(mol_flag_1(project_id.as_bytes32()).as_slice().pack())
        .build()
}

pub fn check_valid_request(cell: &CellOutput, code_hash: &H256) -> bool {
    let lock = &cell.lock();
    if lock.code_hash().as_slice() != code_hash.as_bytes()
        || lock.hash_type() != ScriptHashType::Data.into()
        || !is_mol_flag_2(&lock.args().raw_data())
    {
        return false;
    }
    true
}

pub async fn fill_transaction_capacity_diff(
    rpc: &impl CkbClient,
    lock_script: &Script,
    mut capacity_diff: u64,
    tx: &mut TransactionView,
    outputs: &mut Vec<CellOutput>,
    outputs_data: &mut Vec<Bytes>,
) -> KoResult<()> {
    // append change cell for storing change ckb
    let mut change_cell = CellOutput::new_builder()
        .lock(lock_script.clone())
        .build_exact_capacity(Capacity::zero())
        .unwrap();
    let capacity: u64 = change_cell.capacity().unpack();
    capacity_diff += capacity;

    // search avaliable input cells
    let search_key = SearchKey {
        script: lock_script.clone().into(),
        script_type: ScriptType::Lock,
        filter: None,
    };
    let mut after = None;
    let mut searched_capacity = 0u64;
    let mut cells = vec![];
    while searched_capacity < capacity_diff {
        let result = rpc
            .fetch_live_cells(search_key.clone(), 1, after)
            .await
            .map_err(|_| AssemblerError::MissProjectRequestCell)?;
        let cell = &result.objects[0];
        let capacity: u64 = cell.output.capacity.into();
        searched_capacity += capacity;
        cells.push(
            CellInput::new_builder()
                .previous_output(cell.out_point.clone().into())
                .build(),
        );
        if result.last_cursor.is_empty() {
            break;
        }
        after = Some(result.last_cursor);
    }
    if searched_capacity < capacity_diff {
        return Err(
            AssemblerError::InsufficientCellCapacity(capacity_diff - searched_capacity).into(),
        );
    }

    // complete transaction parts
    change_cell = change_cell
        .as_builder()
        .capacity(Capacity::shannons(searched_capacity - capacity_diff + capacity).pack())
        .build();
    outputs.push(change_cell);
    outputs_data.push(Bytes::new());
    *tx = tx.as_advanced_builder().inputs(cells).build();

    Ok(())
}

pub fn get_extractable_capacity(cell: &CellOutput, data_len: usize) -> u64 {
    let capacity: u64 = cell.capacity().unpack();
    let occupied_capacity: u64 = cell
        .occupied_capacity(Capacity::bytes(data_len).unwrap())
        .unwrap()
        .as_u64();
    capacity - occupied_capacity
}

pub fn clone_with_new_capacity(cell: &CellOutput, capacity: u64) -> CellOutput {
    cell.clone().as_builder().capacity(capacity.pack()).build()
}
