use std::cell::RefCell;
use std::rc::Rc;

use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::packed::Script;
use ko_protocol::types::assembler::KoRequest;
use ko_protocol::types::context::KoContextGlobalCell;
use ko_protocol::types::executor::KoExecutedRequest;
use ko_protocol::{hex, serde_json, KoResult};
use mlua::{Lua, LuaSerdeExt, Table, Value};

use crate::error::ExecutorError;
use crate::luac;

pub fn run_request(
    lua: &Lua,
    owner: &Script,
    global_driver: &mut Script,
    request: &KoRequest,
    offset: usize,
) -> KoExecutedRequest {
    // prepare personal context injections
    let context: Table = luac!(lua.globals().get("KOC"));
    let request_owner = request.lock_script.calc_script_hash();
    luac!(context.set("user", hex::encode(request_owner.raw_data())));
    if let Some(script) = &request.recipient_script {
        let recipient_lockhash = script.calc_script_hash();
        luac!(context.set("recipient", hex::encode(recipient_lockhash.raw_data())));
    }
    if !request.json_data.is_empty() {
        let value: serde_json::Value = serde_json::from_slice(&request.json_data)
            .map_err(|_| ExecutorError::InvalidJsonFormatForPersonalData)?;
        let personal_table = luac!(lua.to_value(&value));
        luac!(context.set("personal", personal_table));
    }
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

    // check specified user lock_hash
    let user_lockhash: mlua::String = luac!(context.get("user"));
    let koc_user: String = luac!(user_lockhash.to_str()).into();
    let expect_user = hex::encode(&request.lock_script.calc_script_hash().raw_data());
    let expect_recipient = if let Some(script) = &request.recipient_script {
        hex::encode(&script.calc_script_hash().raw_data())
    } else {
        String::new()
    };
    let user_lockscript = {
        if koc_user == expect_user {
            request.lock_script.clone()
        } else if request.recipient_script.is_some() && koc_user == expect_recipient {
            request.recipient_script.as_ref().unwrap().clone()
        } else {
            return Err(ExecutorError::UnexpectedUserLockhash.into());
        }
    };

    // make sure global is a table value
    let _global: Table = luac!(context.get("global"));

    // check specified driver lock_hash
    let driver_lockhash: mlua::String = luac!(context.get("driver"));
    let koc_driver: String = luac!(driver_lockhash.to_str()).into();
    let expect_driver = hex::encode(global_driver.calc_script_hash().raw_data());
    if koc_driver != expect_driver {
        *global_driver = {
            if koc_driver == expect_owner {
                owner.clone()
            } else if koc_driver == expect_user {
                request.lock_script.clone()
            } else if request.recipient_script.is_some() && koc_driver == expect_recipient {
                request.recipient_script.as_ref().unwrap().clone()
            } else {
                return Err(ExecutorError::UnexpectedDriverLockhash.into());
            }
        };
    }

    // generate cell_output data
    let output_data: Value = luac!(context.get("personal"));
    let json_data = match output_data {
        Value::Nil => None,
        Value::Table(data) => {
            let data = serde_json::to_string(&data).unwrap();
            Some(Bytes::from(data.as_bytes().to_vec()))
        }
        _ => Err(ExecutorError::ErrorLoadRequestLuaCode(
            String::from_utf8(request.function_call.to_vec()).unwrap(),
            "the return value can only be nil or table".into(),
        ))?,
    };
    Ok((json_data, user_lockscript))
}

pub fn parse_requests_to_outputs(
    lua: &Lua,
    owner: &Script,
    global_cell: &mut KoContextGlobalCell,
    requests: &[KoRequest],
) -> KoResult<Vec<KoExecutedRequest>> {
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
    let global_capacity = Rc::new(RefCell::new(global_cell.capacity));
    let capacity = global_capacity.clone();
    let ckb_withdraw = luac!(lua.create_function(move |_, ckb: f64| {
        let withdraw_ckb = (ckb * 100_000_000.0) as u64;
        let avaliable_ckb = *capacity.borrow() - occupied_ckb;
        if avaliable_ckb >= withdraw_ckb {
            *capacity.borrow_mut() -= withdraw_ckb;
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
                context.clone()
            };
            match run_request(lua, owner, &mut global_cell.lock_script, request, i) {
                Ok(output) => Ok(output),
                Err(err) => {
                    // recover previous global data
                    luac!(lua.globals().set("KOC", previous_context));
                    Err(err)
                }
            }
        })
        .collect::<Vec<_>>();

    // apply adjusted global cell capacity
    global_cell.capacity = *global_capacity.borrow();

    Ok(user_outputs)
}
