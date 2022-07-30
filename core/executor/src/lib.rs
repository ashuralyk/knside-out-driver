use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::H256;
use ko_protocol::derive_more::Constructor;
use ko_protocol::traits::Executor;
use ko_protocol::types::{assembler::KoRequest, executor::KoExecuteReceipt};
use ko_protocol::{serde_json, KoResult};
use mlua::{Lua, LuaSerdeExt, Table, Value};

mod error;
use error::ExecutorError;

macro_rules! luac {
    ($res:expr) => {
        $res.map_err(|err| ExecutorError::from(err))?
    };
}

#[derive(Constructor)]
pub struct ExecutorImpl {}

impl Executor for ExecutorImpl {
    fn execute_lua_requests(
        &self,
        global_json_data: &Bytes,
        project_owner: &H256,
        user_requests: &[KoRequest],
        project_lua_code: &Bytes,
    ) -> KoResult<KoExecuteReceipt> {
        let lua = Lua::new();
        // initialize project lua code
        lua.load(&project_lua_code.to_vec())
            .exec()
            .map_err(|err| ExecutorError::ErrorLoadProjectLuaCode(err.to_string()))?;
        // prepare global context `msg`
        let cost_ckbs = Rc::new(RefCell::new(HashMap::new()));
        let msg = luac!(lua.create_table());
        luac!(msg.set("owner", hex::encode(project_owner.as_bytes())));
        let ckbs = cost_ckbs.clone();
        let ckb_cost = luac!(lua.create_function(move |lua, ckb: u64| {
            let i: usize = lua.globals().get("i").expect("ckb_cost get i");
            ckbs.borrow_mut().insert(i, ckb);
            Ok(true)
        }));
        luac!(msg.set("ckb_cost", ckb_cost));
        let global_table = {
            let json_string = String::from_utf8(global_json_data.to_vec())
                .map_err(|_| ExecutorError::InvalidUTF8FormatForGlobalData)?;
            let value: serde_json::Value = serde_json::from_str(&json_string)
                .map_err(|_| ExecutorError::InvalidJsonFormatForGlobalData)?;
            luac!(lua.to_value(&value))
        };
        luac!(msg.set("global", global_table));
        luac!(lua.globals().set("msg", msg));
        // running each user function_call requests
        let personal_outputs = user_requests
            .iter()
            .enumerate()
            .map(|(i, request)| {
                let msg: Table = luac!(lua.globals().get("msg"));
                let request_owner = request.lock_script.calc_script_hash();
                luac!(msg.set("sender", hex::encode(request_owner.raw_data())));
                let personal_table = {
                    let json_string = String::from_utf8(request.json_data.to_vec())
                        .map_err(|_| ExecutorError::InvalidUTF8FormatForPersonalData)?;
                    if json_string.is_empty() {
                        Value::Table(luac!(lua.create_table()))
                    } else {
                        let value: serde_json::Value = serde_json::from_str(&json_string)
                            .map_err(|_| ExecutorError::InvalidJsonFormatForPersonalData)?;
                        luac!(lua.to_value(&value))
                    }
                };
                luac!(msg.set("data", personal_table));
                luac!(lua.globals().set("msg", msg));
                luac!(lua.globals().set("i", i));
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
                let output_data: Value = luac!(return_table.get("data"));
                // generate cell_output data
                let json_data = match output_data {
                    Value::Nil => None,
                    Value::Table(data) => {
                        let data = serde_json::to_string(&data).unwrap();
                        if data == "{}" {
                            Some(Bytes::new())
                        } else {
                            Some(Bytes::from(data.as_bytes().to_vec()))
                        }
                    }
                    _ => Err(ExecutorError::ErrorLoadRequestLuaCode(
                        String::from_utf8(request.function_call.to_vec()).unwrap(),
                        "the return value can only be nil or table".into(),
                    ))?,
                };
                // TODO: leave code to handle `transfer` owner_lockscript
                let owner_lockscript = request.lock_script.clone();
                Ok((json_data, owner_lockscript))
            })
            .collect::<KoResult<Vec<_>>>()?;
        // check input/output ckbs are wether matched
        user_requests
            .iter()
            .enumerate()
            .try_for_each::<_, KoResult<_>>(|(i, request)| {
                if let Some(payment) = cost_ckbs.borrow().get(&i) {
                    if &request.payment < payment {
                        return Err(ExecutorError::InsufficientRequiredCkb(
                            request.payment,
                            *payment,
                            i,
                        )
                        .into());
                    }
                }
                Ok(())
            })?;
        // make final global json string
        let global_json_data = {
            let msg: Table = luac!(lua.globals().get("msg"));
            let global_table: Table = luac!(msg.get("global"));
            let data = serde_json::to_string(&global_table).unwrap();
            Bytes::from(data.as_bytes().to_vec())
        };
        // collect results to execution receipt
        let receipt = KoExecuteReceipt::new(global_json_data, personal_outputs);
        Ok(receipt)
    }
}
