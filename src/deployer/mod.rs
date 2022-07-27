use ckb_hash::new_blake2b;
use ckb_jsonrpc_types::TransactionView;
use ko_core_driver::DriverImpl;
use ko_protocol::ckb_sdk::constants::TYPE_ID_CODE_HASH;
use ko_protocol::ckb_sdk::rpc::ckb_indexer::{Order, ScriptType, SearchKey};
use ko_protocol::ckb_sdk::IndexerRpcClient;
use ko_protocol::ckb_types::core::{Capacity, DepType, ScriptHashType, TransactionBuilder};
use ko_protocol::ckb_types::packed::{
    CellDep, CellInput, CellOutput, OutPoint, Script, ScriptOpt, WitnessArgs,
};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack, Unpack};
use ko_protocol::ckb_types::{bytes::Bytes, h160, h256, H160, H256};
use ko_protocol::types::generated::{mol_deployment, mol_flag_0};
use ko_protocol::{secp256k1::SecretKey, serde_json, traits::Driver};

const CKB_URL: &str = "http://127.0.0.1:8114";
const CKB_INDEXER_URL: &str = "http://127.0.0.1:8116";

const OWNER_PRIVATE_KEY: H256 =
    h256!("0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df");
const OWNER_PUBKEY_HASH: H160 = h160!("0x58b85c196e5fe80e25b4dab596e7121d219f79fb");

const PROJECT_CODE_HASH: H256 =
    h256!("0xe72d02d5a2dbf1b893e5b9ae17602395533b6b918cee67d234669c24b41cc4f7");
const SECP256K1_CODE_HASH: H256 =
    h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");

const SECP256K1_TX_HASH: H256 =
    h256!("0xc6bffa9ca9a9dadfec83c0307eee18fe88e42a00d05068510d799e3e4ad3ee87");
const KNSIDEOUT_TX_HASH: H256 =
    h256!("0x959d0be02e1e1ef3abca6ebd19bcf386244ab706ac4a5738257fcb27ee07fb6a");

fn build_type_id_script(input: Option<&CellInput>, output_index: u64) -> ScriptOpt {
    let mut ret = [0; 32];
    if let Some(input) = input {
        let mut blake2b = new_blake2b();
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

fn build_secp256k1_script(args: &[u8]) -> Script {
    Script::new_builder()
        .code_hash(SECP256K1_CODE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(args.pack())
        .build()
}

fn build_knsideout_script(args: &[u8]) -> Script {
    Script::new_builder()
        .code_hash(PROJECT_CODE_HASH.pack())
        .hash_type(ScriptHashType::Data.into())
        .args(args.pack())
        .build()
}

#[test]
fn deploy_project_deployment_cell() {
    // make project of output-data
    let lua_code = std::fs::read_to_string("./src/deployer/21-point.lua").unwrap();
    let deployment = mol_deployment(lua_code.as_str());

    // make global of output-data
    let lua = mlua::Lua::new();
    lua.load(&lua_code).exec().expect("exec lua code");
    let func_init_global: mlua::Function = lua.globals().get("construct").expect("get construct");
    let global_data = func_init_global
        .call::<_, mlua::Table>(())
        .expect("call construct");
    let global_data_json = serde_json::to_string(&global_data).unwrap();
    println!("global_data_json = {}", global_data_json);

    // build mock knside-out transaction outputs and data
    let secp256k1_script = build_secp256k1_script(OWNER_PUBKEY_HASH.as_bytes());
    let global_type_script = build_knsideout_script(mol_flag_0(&[0u8; 32]).as_slice());
    let mut outputs = vec![
        // project cell
        CellOutput::new_builder()
            .lock(secp256k1_script.clone())
            .type_(build_type_id_script(None, 0))
            .build_exact_capacity(Capacity::bytes(deployment.as_slice().len()).unwrap())
            .unwrap(),
        // global cell
        CellOutput::new_builder()
            .lock(secp256k1_script.clone())
            .type_(Some(global_type_script).pack())
            .build_exact_capacity(Capacity::bytes(global_data_json.len()).unwrap())
            .unwrap(),
        // change cell
        CellOutput::new_builder()
            .lock(secp256k1_script.clone())
            .build_exact_capacity(Capacity::zero())
            .unwrap(),
    ];
    let outputs_data = vec![
        deployment.as_bytes(),
        Bytes::from(global_data_json.as_bytes().to_vec()),
        Bytes::default()
    ];
    let mut outputs_capacity = outputs
        .iter()
        .map(|output| output.capacity().unpack())
        .collect::<Vec<u64>>()
        .iter()
        .sum::<u64>();
    let fee = Capacity::bytes(1).unwrap().as_u64();
    outputs_capacity += fee;

    // fill knside-out transaction inputs
    let mut inputs = vec![];
    let mut inputs_capacity = 0u64;
    let mut indexer_rpc = IndexerRpcClient::new(CKB_INDEXER_URL);
    let search = SearchKey {
        script: secp256k1_script.into(),
        script_type: ScriptType::Lock.into(),
        filter: None,
    };
    let mut after = None;
    while inputs_capacity < outputs_capacity {
        let result = indexer_rpc
            .get_cells(search.clone(), Order::Asc, 10.into(), after)
            .unwrap();
        result
            .objects
            .into_iter()
            .filter(|cell| cell.output.type_.is_none())
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
    println!(
        "inputs_capacity = {}, outputs_capacity = {}",
        inputs_capacity, outputs_capacity
    );
    assert!(inputs_capacity >= outputs_capacity);

    // rebuild type_id with real input and change
    let project_type_script = build_type_id_script(Some(&inputs[0]), 0);
    let project_type_id = project_type_script.to_opt().unwrap().calc_script_hash();
    outputs[0] = outputs[0]
        .clone()
        .as_builder()
        .type_(project_type_script)
        .build();
    let global_type_script =
        build_knsideout_script(mol_flag_0(&project_type_id.unpack().0).as_slice());
    outputs[1] = outputs[1]
        .clone()
        .as_builder()
        .type_(Some(global_type_script).pack())
        .build();
    let change = inputs_capacity - outputs_capacity;
    outputs[2] = outputs[2]
        .clone()
        .as_builder()
        .build_exact_capacity(Capacity::shannons(change))
        .unwrap();

    // build knside-out transaction celldeps
    let cell_deps = vec![
        CellDep::new_builder()
            .out_point(
                OutPoint::new_builder()
                    .tx_hash(KNSIDEOUT_TX_HASH.pack())
                    .index(0u32.pack())
                    .build(),
            )
            .dep_type(DepType::Code.into())
            .build(),
        CellDep::new_builder()
            .out_point(
                OutPoint::new_builder()
                    .tx_hash(SECP256K1_TX_HASH.pack())
                    .index(0u32.pack())
                    .build(),
            )
            .dep_type(DepType::DepGroup.into())
            .build(),
    ];

    // build knside-out transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_deps(cell_deps)
        .build();

    // sign transaction
    let privkey = SecretKey::from_slice(OWNER_PRIVATE_KEY.as_bytes()).unwrap();
    let mut driver = DriverImpl::new(CKB_URL, &privkey);
    let signature = driver.sign_ko_transaction(&tx);

    // set witnesses
    let witness = WitnessArgs::new_builder()
        .lock(Some(signature).pack())
        .build()
        .as_bytes();
    let tx = tx
        .as_advanced_builder()
        .witnesses(vec![witness].pack())
        .build();
    println!(
        "tx = {}",
        serde_json::to_string_pretty(&TransactionView::from(tx.clone())).unwrap()
    );

    // send knside-out transaction to CKB
    let hash = driver.send_ko_transaction(tx).expect("send tx");
    println!("send success, hash = {}", hash);
}

#[test]
fn update_project_deployment_cell() {

}
