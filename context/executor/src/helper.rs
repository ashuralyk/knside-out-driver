use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::packed::Script;
use ko_protocol::types::assembler::KoRequest;
use ko_protocol::{hex, serde_json, KoResult};
use mlua::{Lua, LuaSerdeExt, Table, Value};

use crate::error::ExecutorError;
use crate::luac;

type ExecuteResult = Vec<KoResult<(Option<Bytes>, Script)>>;

pub fn run_request(
    lua: &Lua,
    request: &KoRequest,
    offset: usize,
) -> KoResult<(Option<Bytes>, Script)> {
    let msg: Table = luac!(lua.globals().get("msg"));
    let request_owner = request.lock_script.calc_script_hash();
    let recipient_owner = {
        if let Some(script) = &request.recipient_script {
            script.calc_script_hash()
        } else {
            request_owner.clone()
        }
    };
    luac!(msg.set("sender", hex::encode(request_owner.raw_data())));
    luac!(msg.set("recipient", hex::encode(recipient_owner.raw_data())));
    if !request.json_data.is_empty() {
        let value: serde_json::Value = serde_json::from_slice(&request.json_data)
            .map_err(|_| ExecutorError::InvalidJsonFormatForPersonalData)?;
        let personal_table = luac!(lua.to_value(&value));
        luac!(msg.set("data", personal_table));
    }
    luac!(lua.globals().set("msg", msg));
    luac!(lua.globals().set("i", offset));
    let function_call = {
        let mut call = b"return ".to_vec();
        call.append(&mut request.function_call.to_vec());
        call
    };
    let return_table: Table = lua.load(&function_call).call(()).map_err(|err| {
        ExecutorError::ErrorLoadRequestLuaCode(
            String::from_utf8(request.function_call.to_vec()).unwrap(),
            err.to_string(),
        )
    })?;
    // check specified owner lock_hash
    let owner_lockhash: mlua::String = luac!(return_table.get("owner"));
    let owner_lockscript = {
        if let Some(script) = &request.recipient_script {
            script.clone()
        } else {
            request.lock_script.clone()
        }
    };
    let lua_owner = luac!(owner_lockhash.to_str()).into();
    let rust_owner = hex::encode(&owner_lockscript.calc_script_hash().raw_data());
    if lua_owner != rust_owner {
        return Err(ExecutorError::OwnerLockhashMismatch(lua_owner, rust_owner).into());
    }
    // generate cell_output data
    let output_data: Value = luac!(return_table.get("data"));
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
    Ok((json_data, owner_lockscript))
}

pub fn parse_requests_to_outputs(lua: &Lua, requests: &[KoRequest]) -> KoResult<ExecuteResult> {
    let msg: Table = luac!(lua.globals().get("msg"));
    let cost_ckbs = Rc::new(RefCell::new(HashMap::new()));
    let ckbs = cost_ckbs.clone();
    let ckb_cost = luac!(lua.create_function(move |lua, ckb: u64| {
        let i: usize = lua.globals().get("i").expect("ckb_cost get i");
        ckbs.borrow_mut().insert(i, ckb);
        Ok(true)
    }));
    luac!(msg.set("ckb_cost", ckb_cost));
    luac!(lua.globals().set("msg", msg));
    let user_outputs = requests
        .iter()
        .enumerate()
        .map(|(i, request)| {
            let output = run_request(lua, request, i)?;
            // check cell_capacity
            if let Some(require) = cost_ckbs.borrow().get(&i) {
                let offer = &request.payment;
                if offer < require {
                    return Err(ExecutorError::InsufficientRequiredCkb(*offer, *require, i).into());
                }
            }
            Ok(output)
        })
        .collect::<Vec<_>>();
    Ok(user_outputs)
}
