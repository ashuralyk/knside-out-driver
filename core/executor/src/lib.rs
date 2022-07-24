use std::rc::Rc;
use std::cell::RefCell;

use ko_protocol::traits::Executor;
use ko_protocol::ckb_types::H256;
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::types::{assembler::KoRequest, executor::KoExecuteReceipt};
use ko_protocol::KoResult;
use ko_protocol::serde_json::to_string;
use mlua::{Lua, Table};

mod error;
use error::ExecutorError;

pub struct ExecutorImpl {}

impl Executor for ExecutorImpl {
    fn execute_lua_requests (
        &self,
        global_json_data: &Bytes,
        project_owner: &H256,
        user_requests: &Vec<KoRequest>,
        project_lua_code: &Bytes
    ) -> KoResult<KoExecuteReceipt> {
        let lua = Lua::new();
        // initialize project lua code
        lua.load(&project_lua_code.to_vec()).exec().map_err(|err| {
            ExecutorError::ErrorLoadProjectLuaCode(err.to_string())
        })?;
        // prepare global context `msg`
        let required_ckbs = Rc::new(RefCell::new(vec![]));
        let msg = lua.create_table().unwrap();
        msg.set("owner", hex::encode(project_owner.as_bytes())).unwrap();
        let ckbs = required_ckbs.clone();
        let ckb_cost = lua.create_function(move |_, ckb: u64| {
            ckbs.borrow_mut().push(ckb);
            Ok(true)
        }).unwrap();
        msg.set("ckb_cost", ckb_cost).unwrap();
        let global_json_string = String::from_utf8(global_json_data.to_vec())
            .map_err(|_| ExecutorError::InvalidUTF8FormatForGlobalData)?;
        msg.set("global", global_json_string).unwrap();
        lua.globals().set("msg", msg).unwrap();
        // running each user function_call requests
        let outputs_json_data = user_requests
            .iter()
            .map(|request| {
                let msg: Table = lua.globals().get("msg").unwrap();
                let request_owner = request.lock_script.calc_script_hash();
                msg.set("sender", hex::encode(request_owner.raw_data())).unwrap();
                let personal_json_string = String::from_utf8(request.json_data.to_vec())
                    .map_err(|_| ExecutorError::InvalidUTF8FormatForPersonalData)?;
                msg.set("data", personal_json_string).unwrap();
                lua.globals().set("msg", msg).unwrap();
                let output_data: Table = lua.load(&request.function_call.to_vec()).call(()).map_err(|err| {
                    ExecutorError::ErrorLoadRequestLuaCode(
                        String::from_utf8(request.function_call.to_vec()).unwrap(),
                        err.to_string()
                    )
                })?;
                Ok(to_string(&output_data).unwrap())
            })
            .collect::<KoResult<Vec<_>>>()?;
        // collect running results
        let receipt = KoExecuteReceipt::new(
            outputs_json_data,
            required_ckbs.borrow().clone()
        );
        Ok(receipt)
    }
}
