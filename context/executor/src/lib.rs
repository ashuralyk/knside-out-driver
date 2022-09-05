use std::cell::RefCell;
use std::rc::Rc;

use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::derive_more::Constructor;
use ko_protocol::traits::Executor;
use ko_protocol::types::{assembler::KoRequest, executor::KoExecuteReceipt};
use ko_protocol::{hex, serde_json, KoResult, H256};
use mlua::{Function, Lua, LuaSerdeExt, Table};

mod error;
mod helper;
use error::ExecutorError;

#[macro_export]
macro_rules! luac {
    ($res:expr) => {
        $res.map_err(|err| ExecutorError::from(err))?
    };
}

#[derive(Constructor)]
pub struct ExecutorImpl {}

impl ExecutorImpl {
    fn prepare_lua_context(
        &self,
        global_json_data: &Bytes,
        project_owner: &H256,
        project_lua_code: &Bytes,
    ) -> KoResult<Lua> {
        // initialize project lua code
        let lua = Lua::new();
        lua.load(&project_lua_code.to_vec())
            .exec()
            .map_err(|err| ExecutorError::ErrorLoadProjectLuaCode(err.to_string()))?;

        // prepare global context `msg`
        let msg = luac!(lua.create_table());
        luac!(msg.set("owner", hex::encode(project_owner.as_bytes())));
        let global_table = {
            let json_string = String::from_utf8(global_json_data.to_vec())
                .map_err(|_| ExecutorError::InvalidUTF8FormatForGlobalData)?;
            let value: serde_json::Value = serde_json::from_str(&json_string)
                .map_err(|_| ExecutorError::InvalidJsonFormatForGlobalData(json_string))?;
            luac!(lua.to_value(&value))
        };
        luac!(msg.set("global", global_table));
        luac!(lua.globals().set("msg", msg));

        Ok(lua)
    }
}

impl Executor for ExecutorImpl {
    fn execute_lua_requests(
        &self,
        global_json_data: &Bytes,
        project_owner: &H256,
        user_requests: &[KoRequest],
        project_lua_code: &Bytes,
        random_seeds: &[i64; 2],
    ) -> KoResult<KoExecuteReceipt> {
        let lua = self.prepare_lua_context(global_json_data, project_owner, project_lua_code)?;
        let math: Table = luac!(lua.globals().get("math"));
        let randomseed: Function = luac!(math.get("randomseed"));
        luac!(randomseed.call((random_seeds[0], random_seeds[1])));

        // running each user function_call requests
        let personal_outputs = helper::parse_requests_to_outputs(&lua, user_requests)?;

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

    fn estimate_payment_ckb(
        &self,
        global_json_data: &Bytes,
        project_owner: &H256,
        request: KoRequest,
        project_lua_code: &Bytes,
    ) -> KoResult<u64> {
        let lua = self.prepare_lua_context(global_json_data, project_owner, project_lua_code)?;

        // prepare payment ckb catcher
        let msg: Table = luac!(lua.globals().get("msg"));
        let payment_ckb = Rc::new(RefCell::new(0u64));
        let payment = payment_ckb.clone();
        let ckb_cost = luac!(lua.create_function(move |_, ckb: f64| {
            *payment.borrow_mut() = (ckb * 100_000_000.0) as u64;
            Ok(true)
        }));
        luac!(msg.set("ckb_cost", ckb_cost));
        luac!(lua.globals().set("msg", msg));

        // run request and trigger ckb_cost function if it exists
        helper::run_request(&lua, &request, 0)?;

        // get payment ckb cost
        Ok(payment_ckb.take())
    }
}
