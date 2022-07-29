use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::H256;
use ko_protocol::serde_json::to_string;
use ko_protocol::traits::Executor;
use ko_protocol::types::{assembler::KoRequest, executor::KoExecuteReceipt};
use ko_protocol::KoResult;
use mlua::{Lua, Table};

mod error;
use error::ExecutorError;

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
        let msg = lua.create_table().unwrap();
        msg.set("owner", hex::encode(project_owner.as_bytes()))
            .unwrap();
        let ckbs = cost_ckbs.clone();
        let ckb_cost = lua
            .create_function(move |lua, ckb: u64| {
                let i: usize = lua.globals().get("i").unwrap();
                ckbs.borrow_mut().insert(i, ckb);
                Ok(true)
            })
            .unwrap();
        msg.set("ckb_cost", ckb_cost).unwrap();
        let global_json_string = String::from_utf8(global_json_data.to_vec())
            .map_err(|_| ExecutorError::InvalidUTF8FormatForGlobalData)?;
        msg.set("global", global_json_string).unwrap();
        lua.globals().set("msg", msg).unwrap();
        // running each user function_call requests
        let outputs_json_data = user_requests
            .iter()
            .enumerate()
            .map(|(i, request)| {
                let msg: Table = lua.globals().get("msg").unwrap();
                let request_owner = request.lock_script.calc_script_hash();
                msg.set("sender", hex::encode(request_owner.raw_data()))
                    .unwrap();
                let personal_json_string = String::from_utf8(request.json_data.to_vec())
                    .map_err(|_| ExecutorError::InvalidUTF8FormatForPersonalData)?;
                msg.set("data", personal_json_string).unwrap();
                lua.globals().set("msg", msg).unwrap();
                let output_data: Table = lua
                    .load(&request.function_call.to_vec())
                    .call(())
                    .map_err(|err| {
                        ExecutorError::ErrorLoadRequestLuaCode(
                            String::from_utf8(request.function_call.to_vec()).unwrap(),
                            err.to_string(),
                        )
                    })?;
                lua.globals().set("i", i).unwrap();
                let json_data = to_string(&output_data).unwrap();
                Ok(Bytes::from(json_data.as_bytes().to_vec()))
            })
            .collect::<KoResult<Vec<_>>>()?;
        // check input/output ckbs are wether matched
        let required_payments = user_requests
            .iter()
            .enumerate()
            .map(|(i, request)| {
                if let Some(payment) = cost_ckbs.borrow().get(&i) {
                    if &request.payment < payment {
                        return Err(ExecutorError::InsufficientRequiredCkb(
                            request.payment,
                            *payment,
                            i,
                        )
                        .into());
                    }
                    Ok(*payment)
                } else {
                    Ok(0u64)
                }
            })
            .collect::<KoResult<_>>()?;
        // collect running results
        Ok(KoExecuteReceipt::new(outputs_json_data, required_payments))
    }
}
