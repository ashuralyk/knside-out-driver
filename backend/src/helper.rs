use std::str::FromStr;

use ckb_hash::blake2b_256;
use ko_protocol::ckb_sdk::constants::TYPE_ID_CODE_HASH;
use ko_protocol::ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey};
use ko_protocol::ckb_sdk::HumanCapacity;
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::core::{ScriptHashType, TransactionView};
use ko_protocol::ckb_types::packed::{
    CellInput, CellOutput, OutPoint, Script, ScriptOpt, Transaction, WitnessArgs,
};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack, Unpack};
use ko_protocol::{mol_identity, traits::CkbClient, KoResult, H256};
use ko_protocol::{mol_request, serde_json};

use crate::BackendError;

pub fn build_knsideout_script(code_hash: &H256, args: &[u8]) -> Script {
    Script::new_builder()
        .code_hash(code_hash.pack())
        .hash_type(ScriptHashType::Data.into())
        .args(args.pack())
        .build()
}

pub fn build_global_type_script(project_code_hash: &H256, project_type_args: &H256) -> Script {
    let project_id: H256 = Script::new_builder()
        .code_hash(TYPE_ID_CODE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(project_type_args.as_bytes().pack())
        .build()
        .calc_script_hash()
        .unpack();
    Script::new_builder()
        .code_hash(project_code_hash.pack())
        .hash_type(ScriptHashType::Data.into())
        .args(mol_identity(0, project_id.as_bytes32()).as_slice().pack())
        .build()
}

pub fn build_type_id_script(input: Option<&CellInput>, output_index: u64) -> ScriptOpt {
    let mut ret = [0; 32];
    if let Some(input) = input {
        let mut blake2b = ckb_hash::new_blake2b();
        blake2b.update(input.as_slice());
        blake2b.update(&output_index.to_le_bytes());
        blake2b.finalize(&mut ret);
    }
    Some(
        Script::new_builder()
            .code_hash(TYPE_ID_CODE_HASH.pack())
            .hash_type(ScriptHashType::Type.into())
            .args(Bytes::from(ret.to_vec()).pack())
            .build(),
    )
    .pack()
}

pub fn recover_type_id_script(args: &[u8]) -> Script {
    Script::new_builder()
        .code_hash(TYPE_ID_CODE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(args.pack())
        .build()
}

pub async fn fetch_live_cells(
    rpc: &impl CkbClient,
    search_key: SearchKey,
    mut inputs_capacity: u64,
    outputs_capacity: u64,
) -> KoResult<(Vec<CellInput>, u64)> {
    let mut inputs = vec![];
    let mut after = None;
    while inputs_capacity < outputs_capacity {
        let result = rpc
            .fetch_live_cells(search_key.clone(), 10, after)
            .await
            .map_err(|err| BackendError::IndexerRpcError(err.to_string()))?;
        result
            .objects
            .into_iter()
            .filter(|cell| cell.output.type_.is_none() && cell.output_data.is_empty())
            .for_each(|cell| {
                if inputs_capacity < outputs_capacity {
                    inputs.push(
                        CellInput::new_builder()
                            .previous_output(cell.out_point.into())
                            .build(),
                    );
                    inputs_capacity += u64::from(cell.output.capacity);
                }
            });
        if result.last_cursor.is_empty() {
            break;
        }
        after = Some(result.last_cursor);
    }
    Ok((inputs, inputs_capacity))
}

pub async fn fetch_cell_by_script(
    rpc: &impl CkbClient,
    lock_script: &Script,
) -> KoResult<(CellInput, u64)> {
    let search = SearchKey {
        script: lock_script.clone().into(),
        script_type: ScriptType::Lock,
        filter: None,
    };
    let (cells, ckb) = fetch_live_cells(rpc, search, 0, 0).await?;
    if cells.is_empty() {
        return Err(BackendError::MissInputCell.into());
    }
    Ok((cells[0].clone(), ckb))
}

pub async fn fetch_outpoint_cell(
    rpc: &impl CkbClient,
    out_point: &OutPoint,
) -> KoResult<(CellOutput, String, [u8; 32])> {
    let tx: Transaction = rpc
        .get_transaction(&out_point.tx_hash().unpack())
        .await
        .map_err(|err| BackendError::CkbRpcError(err.to_string()))?
        .unwrap()
        .transaction
        .unwrap()
        .inner
        .into();
    let tx = tx.into_view();
    let index: u32 = out_point.index().unpack();
    let cell = tx.output_with_data(index as usize);
    if let Some((cell, data)) = cell {
        let data_hash = blake2b_256(&data);
        let data = String::from_utf8(data.to_vec()).map_err(|_| BackendError::InvalidCell)?;
        Ok((cell, data, data_hash))
    } else {
        Err(BackendError::InvalidCell.into())
    }
}

pub fn get_transaction_digest(tx: &TransactionView) -> H256 {
    let mut blake2b = ckb_hash::new_blake2b();
    blake2b.update(&tx.hash().raw_data());
    // prepare empty witness for digest
    let witness_for_digest = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(vec![0u8; 65])).pack())
        .build();
    // hash witness message
    let mut message = [0u8; 32];
    let witness_len = witness_for_digest.as_bytes().len() as u64;
    blake2b.update(&witness_len.to_le_bytes());
    blake2b.update(&witness_for_digest.as_bytes());
    blake2b.finalize(&mut message);
    message.into()
}

pub fn parse_contract_code(contract: &Bytes) -> KoResult<Vec<u8>> {
    let lua = mlua::Lua::new();
    let function = lua
        .load(contract.as_ref())
        .into_function()
        .map_err(|err| BackendError::BadContractByteCode(err.to_string()))?;
    Ok(function.dump(true))
}

pub fn get_global_json_data(
    contract: &Bytes,
    contract_owner: &String,
    driver_manager: &String,
) -> KoResult<(String, bool, Vec<u8>)> {
    let lua = mlua::Lua::new();
    let function = lua
        .load(contract.as_ref())
        .into_function()
        .map_err(|err| BackendError::BadContractByteCode(err.to_string()))?;
    function
        .call::<_, ()>(())
        .map_err(|err| BackendError::BadContractByteCode(err.to_string()))?;
    let func_init_global: mlua::Function = lua
        .globals()
        .get("construct")
        .map_err(|err| BackendError::ConstructFunctionError(err.to_string()))?;
    let context = {
        let table = lua
            .create_table()
            .map_err(|err| BackendError::CreateKOCTableError(err.to_string()))?;
        table
            .set("owner", contract_owner.clone())
            .map_err(|err| BackendError::CreateKOCTableError(err.to_string()))?;
        table
            .set("driver", driver_manager.clone())
            .map_err(|err| BackendError::CreateKOCTableError(err.to_string()))?;
        table
    };
    lua.globals()
        .set("KOC", context)
        .map_err(|err| BackendError::InjectKOCContextError(err.to_string()))?;
    let global_driver_data = func_init_global
        .call::<_, mlua::Table>(())
        .map_err(|err| BackendError::ConstructFunctionError(err.to_string()))?;
    // check contract driver selection
    let global_driver: String = global_driver_data
        .get("driver")
        .map_err(|err| BackendError::InvalidConstructReturnType(err.to_string()))?;
    if &global_driver != contract_owner || &global_driver != driver_manager {
        return Err(BackendError::InvalidSpecificContractDriver.into());
    }
    // parse json format global data
    let global_data: mlua::Table = global_driver_data
        .get("global")
        .map_err(|err| BackendError::InvalidConstructReturnType(err.to_string()))?;
    let global_data_json = serde_json::to_string(&global_data)
        .map_err(|err| BackendError::GlobalTableNotJsonify(err.to_string()))?;
    let dump = function.dump(true);
    println!("len = {}", dump.len());
    Ok((global_data_json, &global_driver == contract_owner, dump))
}

pub fn calc_outputs_capacity(outputs: &[CellOutput], fee: &str) -> u64 {
    let mut outputs_capacity = outputs
        .iter()
        .map(|output| output.capacity().unpack())
        .collect::<Vec<u64>>()
        .iter()
        .sum::<u64>();
    let fee = HumanCapacity::from_str(fee).unwrap();
    outputs_capacity += fee.0;
    outputs_capacity
}

pub fn complete_transaction_with_signature(
    tx: TransactionView,
    signature: &[u8; 65],
) -> TransactionView {
    let witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(signature.to_vec())).pack())
        .build()
        .as_bytes();
    tx.as_advanced_builder()
        .witnesses(vec![witness].pack())
        .build()
}

pub fn make_request_data(
    method: &str,
    cells: &[(Script, String)],
    cell_deps: &[(&OutPoint, [u8; 32])],
    floatings: &[Script],
) -> Vec<u8> {
    let cells = cells
        .iter()
        .map(|(lock, data)| {
            let data = if data.is_empty() {
                None
            } else {
                Some(data.as_bytes())
            };
            (lock.as_slice(), data)
        })
        .collect::<Vec<_>>();
    let cell_deps = cell_deps
        .iter()
        .map(|(out_point, data_hash)| {
            let hash: H256 = out_point.tx_hash().unpack();
            let index: u32 = out_point.index().unpack();
            (hash, index, data_hash)
        })
        .collect::<Vec<_>>();
    let cell_deps = cell_deps
        .iter()
        .map(|(hash, index, data_hash)| (hash.as_bytes32(), *index as u8, *data_hash))
        .collect::<Vec<_>>();
    let floatings = floatings
        .iter()
        .map(|lock| lock.as_slice())
        .collect::<Vec<_>>();
    mol_request(method, &cells, &cell_deps, &floatings)
}
