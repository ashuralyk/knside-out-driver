use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::packed::Script;
use ko_protocol::types::assembler::{KoCellOutput, KoRequest};
use ko_protocol::types::context::KoContextGlobalCell;
use ko_protocol::{hex, serde_json, KoResult};
use mlua::{Lua, LuaSerdeExt, Table, Value};

use crate::error::ExecutorError;
use crate::luac;

fn koc_fill_candidates(context: &Table, candidates: &[Script]) -> KoResult<()> {
    let candidates = candidates
        .iter()
        .map(|v| hex::encode(v.calc_script_hash().raw_data()))
        .collect::<Vec<_>>();
    luac!(context.set("candidates", candidates));
    Ok(())
}

fn koc_fill_inputs(lua: &Lua, context: &Table, inputs: &[(Script, Bytes)]) -> KoResult<()> {
    let inputs = inputs
        .iter()
        .map(|(script, data)| {
            let input = luac!(lua.create_table());
            let owner = hex::encode(script.calc_script_hash().raw_data());
            luac!(input.set("owner", owner));
            if !data.is_empty() {
                let value: serde_json::Value = serde_json::from_slice(data)
                    .map_err(|_| ExecutorError::InvalidJsonFormatForPersonalData)?;
                let data = luac!(lua.to_value(&value));
                luac!(input.set("data", data));
            }
            Ok(input)
        })
        .collect::<KoResult<Vec<_>>>()?;
    luac!(context.set("inputs", inputs));
    Ok(())
}

fn koc_fill_components(lua: &Lua, context: &Table, components: &[Bytes]) -> KoResult<()> {
    let components = components
        .iter()
        .map(|data| {
            let value: serde_json::Value = serde_json::from_slice(data)
                .map_err(|_| ExecutorError::InvalidJsonFormatForCelldepData)?;
            Ok(luac!(lua.to_value(&value)))
        })
        .collect::<KoResult<Vec<_>>>()?;
    luac!(context.set("components", components));
    Ok(())
}

fn koc_extract_outputs(
    context: &Table,
    method_call: &Bytes,
) -> KoResult<Vec<(String, Option<Bytes>)>> {
    let outputs: mlua::Table = luac!(context.get("outputs"));
    let outputs = outputs
        .sequence_values::<mlua::Table>()
        .map(|table| {
            let table = luac!(table);
            let output_owner = {
                let owner: mlua::String = luac!(table.get("owner"));
                luac!(owner.to_str()).into()
            };
            let output_data = {
                let value: Value = luac!(table.get("data"));
                match value {
                    Value::Nil => None,
                    Value::Table(data) => {
                        let data = serde_json::to_string(&data).unwrap();
                        Some(Bytes::from(data.as_bytes().to_vec()))
                    }
                    _ => Err(ExecutorError::ErrorLoadRequestLuaCode(
                        String::from_utf8(method_call.to_vec()).unwrap(),
                        "the output_data can only be nil or table".into(),
                    ))?,
                }
            };
            Ok((output_owner, output_data))
        })
        .collect::<KoResult<Vec<_>>>()?;
    Ok(outputs)
}

fn get_avaliable_users<'a>(
    inputs: &'a [(Script, Bytes)],
    candidates: &'a [Script],
) -> HashMap<String, &'a Script> {
    let mut users = HashMap::new();
    inputs.iter().for_each(|(script, _)| {
        let key = hex::encode(script.calc_script_hash().raw_data());
        users.insert(key, script);
    });
    candidates.iter().for_each(|script| {
        let key = hex::encode(script.calc_script_hash().raw_data());
        users.insert(key, script);
    });
    users
}

pub fn run_request(
    lua: &Lua,
    owner: &Script,
    global_driver: &mut Script,
    request: &KoRequest,
    offset: usize,
) -> KoResult<KoCellOutput> {
    // prepare personal context injections
    let context: Table = luac!(lua.globals().get("KOC"));
    koc_fill_candidates(&context, &request.candidates)?;
    koc_fill_inputs(lua, &context, &request.inputs)?;
    koc_fill_components(lua, &context, &request.components)?;
    luac!(lua.globals().set("KOC", context));
    luac!(lua.globals().set("i", offset));

    // run user request call
    lua.load(&request.function_call.to_vec())
        .call(())
        .map_err(|err| {
            ExecutorError::ErrorLoadRequestLuaCode(
                String::from_utf8(request.function_call.to_vec()).unwrap(),
                err.to_string(),
            )
        })?;

    // check specified owner lock_hash
    let context: Table = luac!(lua.globals().get("KOC"));
    let owner_lockhash: mlua::String = luac!(context.get("owner"));
    let koc_owner = luac!(owner_lockhash.to_str()).into();
    let expect_owner = hex::encode(owner.calc_script_hash().raw_data());
    if koc_owner != expect_owner {
        return Err(ExecutorError::OwnerLockhashMismatch(koc_owner, expect_owner).into());
    }

    // ure global is a table value
    let _global: Table = luac!(context.get("global"));

    // check specified driver lock_hash
    let expect_users = get_avaliable_users(&request.inputs, &request.candidates);
    let driver_lockhash: mlua::String = luac!(context.get("driver"));
    let koc_driver: String = luac!(driver_lockhash.to_str()).into();
    let expect_driver = hex::encode(global_driver.calc_script_hash().raw_data());
    if koc_driver != expect_driver {
        *global_driver = if let Some(driver) = expect_users.get(&koc_driver) {
            (*driver).clone()
        } else {
            return Err(ExecutorError::UnexpectedDriverLockhash.into());
        }
    }

    // check specified user ouputs
    let outputs = koc_extract_outputs(&context, &request.function_call)?
        .iter()
        .map(|(owner, data)| {
            if let Some(script) = expect_users.get(owner) {
                Ok(((*script).clone(), data.clone()))
            } else {
                Err(ExecutorError::UnexpectedUserOutputLockhash.into())
            }
        })
        .collect::<KoResult<Vec<_>>>()?;

    // make occupied request cell capacity assign to output_cell's basic capacity
    let basic_ckb = request.capacity - request.payment_ckb;
    Ok(KoCellOutput::new(outputs, basic_ckb))
}

pub fn parse_requests_to_outputs(
    lua: &Lua,
    owner: &Script,
    global_cell: &mut KoContextGlobalCell,
    requests: &[KoRequest],
) -> KoResult<Vec<KoResult<KoCellOutput>>> {
    let context: Table = luac!(lua.globals().get("KOC"));

    // complete deposit injection
    let payments = requests.iter().map(|v| v.payment_ckb).collect::<Vec<_>>();
    let ckb_deposit = luac!(lua.create_function(move |lua, ckb: f64| {
        let i: usize = lua.globals().get("i").expect("ckb_deposit get i");
        let offer_ckb = payments.get(i).expect("requests get i");
        let require_ckb = (ckb * 100_000_000.0) as u64;
        Ok(*offer_ckb >= require_ckb)
    }));
    luac!(context.set("ckb_deposit", ckb_deposit));

    // complete withdraw injection
    let occupied_ckb = global_cell.occupied_capacity;
    let global_capacity_rc = Rc::new(RefCell::new(global_cell.capacity));
    let global_capacity = global_capacity_rc.clone();
    let personal_extra_rc = Rc::new(RefCell::new(HashMap::new()));
    let personal_extra = personal_extra_rc.clone();
    let ckb_withdraw = luac!(lua.create_function(move |lua, ckb: f64| {
        let withdraw_ckb = (ckb * 100_000_000.0) as u64;
        let avaliable_ckb = *global_capacity.borrow() - occupied_ckb;
        if avaliable_ckb >= withdraw_ckb {
            *global_capacity.borrow_mut() -= withdraw_ckb;
            let i: usize = lua.globals().get("i").expect("ckb_withdraw get i");
            personal_extra.borrow_mut().insert(i, withdraw_ckb);
            Ok(true)
        } else {
            Ok(false)
        }
    }));
    luac!(context.set("ckb_withdraw", ckb_withdraw));
    luac!(lua.globals().set("KOC", context));

    // transform user requests into transaction cell_outputs
    let user_outputs = requests
        .iter()
        .enumerate()
        .map(|(i, request)| {
            let previous_context = {
                let context: Table = luac!(lua.globals().get("KOC"));
                deep_clone_table(lua, context)?
            };
            match run_request(lua, owner, &mut global_cell.lock_script, request, i) {
                Ok(mut output) => {
                    output.capacity += if let Some(extra_ckb) = personal_extra_rc.borrow().get(&i) {
                        *extra_ckb
                    } else {
                        0
                    };
                    Ok(output)
                }
                Err(err) => {
                    // recover previous global data
                    luac!(lua.globals().set("KOC", previous_context));
                    Err(err)
                }
            }
        })
        .collect::<Vec<_>>();

    // apply adjusted global cell capacity
    global_cell.capacity = *global_capacity_rc.borrow();

    Ok(user_outputs)
}

pub fn deep_clone_table<'lua>(lua: &'lua Lua, table: Table<'lua>) -> KoResult<Table<'lua>> {
    let deep_copy: mlua::Function = luac!(lua.globals().get("_deep_copy"));
    let table = luac!(deep_copy.call::<_, Table<'lua>>(table));
    Ok(table)
}
