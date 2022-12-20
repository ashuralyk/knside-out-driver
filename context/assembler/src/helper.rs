use ko_protocol::ckb_sdk::constants::TYPE_ID_CODE_HASH;
use ko_protocol::ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey, SearchKeyFilter};
use ko_protocol::ckb_sdk::traits::LiveCell;
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::core::{Capacity, ScriptHashType, TransactionView};
use ko_protocol::ckb_types::packed::{CellInput, CellOutput, Script, Transaction};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack, Unpack};
use ko_protocol::generated::Request;
use ko_protocol::traits::CkbClient;
use ko_protocol::types::assembler::KoCellOutput;
use ko_protocol::{is_mol_request, is_mol_request_identity, mol_identity, KoResult, H256};

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
        .args(mol_identity(0, project_id.as_bytes32()).as_slice().pack())
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
        .args(mol_identity(0, project_id.as_bytes32()).as_slice().pack())
        .build()
}

pub fn make_personal_script(code_hash: &H256, project_id: &H256) -> Script {
    Script::new_builder()
        .code_hash(code_hash.pack())
        .hash_type(ScriptHashType::Data.into())
        .args(mol_identity(1, project_id.as_bytes32()).as_slice().pack())
        .build()
}

pub fn check_valid_request(cell: &CellOutput, data: &[u8], code_hash: &H256) -> bool {
    let lock = &cell.lock();
    if lock.code_hash().as_slice() != code_hash.as_bytes()
        || lock.hash_type() != ScriptHashType::Data.into()
        || !is_mol_request_identity(&lock.args().raw_data())
        || !is_mol_request(data)
    {
        return false;
    }
    true
}

pub fn extract_inputs_from_request(request: &Request) -> KoResult<Vec<(Script, Bytes)>> {
    let cells = request
        .cells()
        .into_iter()
        .map(|cell| {
            let script = Script::from_slice(&cell.owner_lockscript().raw_data())
                .map_err(|_| AssemblerError::UnsupportedCallerScriptFormat)?;
            let data = if cell.data().is_none() {
                Bytes::new()
            } else {
                cell.data().to_opt().unwrap().raw_data()
            };
            Ok((script, data))
        })
        .collect::<KoResult<_>>()?;
    Ok(cells)
}

pub fn extract_candidates_from_request(request: &Request) -> KoResult<Vec<Script>> {
    let floatings = request
        .floating_lockscripts()
        .into_iter()
        .map(|floating| {
            Ok(Script::from_slice(&floating.raw_data())
                .map_err(|_| AssemblerError::UnsupportedRecipientScriptFormat)?)
        })
        .collect::<KoResult<_>>()?;
    Ok(floatings)
}

pub async fn extract_components_from_request(
    rpc: &impl CkbClient,
    request: &Request,
) -> KoResult<Vec<Bytes>> {
    let mut celldeps = vec![];
    for celldep in request.function_celldeps().into_iter() {
        let hash = {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(celldep.tx_hash().as_slice());
            hash
        };
        let tx: Transaction = rpc
            .get_transaction(&hash.into())
            .await
            .map_err(|err| AssemblerError::CkbRpcError(err.to_string()))?
            .unwrap()
            .transaction
            .unwrap()
            .inner
            .into();
        let tx = tx.into_view();
        let index: u8 = celldep.index().into();
        let data = tx.outputs_data().get(index as usize);
        if let Some(data) = data {
            celldeps.push(data.unpack());
        } else {
            return Err(AssemblerError::InvalidFunctionCelldep.into());
        }
    }
    Ok(celldeps)
}

pub fn process_raw_outputs(
    i: usize,
    cell_output: &KoCellOutput,
    project_code_hash: &H256,
    project_id: &H256,
) -> (Vec<CellOutput>, Vec<Bytes>, u64) {
    let mut outputs_cell = vec![];
    let mut outputs_capacity = 0u64;
    let mut outputs_data = vec![];
    cell_output.cells.iter().for_each(|(script, data)| {
        let mut cell_output = CellOutput::new_builder()
            .lock(script.clone())
            .build_exact_capacity(Capacity::zero())
            .unwrap();
        let mut cell_output_data = Bytes::new();

        // handle cell which will contain personal json data
        if let Some(data) = data {
            let type_ = if i == 0 {
                make_global_script(project_code_hash, project_id)
            } else {
                make_personal_script(project_code_hash, project_id)
            };
            cell_output = cell_output
                .as_builder()
                .type_(Some(type_).pack())
                .build_exact_capacity(Capacity::bytes(data.len()).unwrap())
                .unwrap();
            cell_output_data = data.clone();
        }

        // record details
        let capacity: u64 = cell_output.capacity().unpack();
        outputs_capacity += capacity;
        outputs_cell.push(cell_output);
        outputs_data.push(cell_output_data);
    });

    // all extra capacity should be added to first cell
    if cell_output.suggested_capacity > outputs_capacity && !outputs_cell.is_empty() {
        let extra_ckb = cell_output.suggested_capacity - outputs_capacity;
        let previous_ckb: u64 = outputs_cell[0].capacity().unpack();
        outputs_cell[0] = outputs_cell[0]
            .clone()
            .as_builder()
            .capacity((previous_ckb + extra_ckb).pack())
            .build();
        outputs_capacity = cell_output.suggested_capacity;
    }
    (outputs_cell, outputs_data, outputs_capacity)
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
        if cell.output.type_.is_some() {
            after = Some(result.last_cursor);
            continue;
        }
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
    assert!(capacity >= occupied_capacity, "global cell capacity error");
    capacity - occupied_capacity
}

pub fn clone_with_new_capacity(cell: &CellOutput, capacity: u64) -> CellOutput {
    cell.clone().as_builder().capacity(capacity.pack()).build()
}
