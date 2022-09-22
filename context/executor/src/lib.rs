use std::cell::RefCell;
use std::rc::Rc;

use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::packed::Script;
use ko_protocol::derive_more::Constructor;
use ko_protocol::traits::Executor;
use ko_protocol::types::assembler::KoRequest;
use ko_protocol::types::context::KoContextGlobalCell;
use ko_protocol::types::executor::KoExecutedRequest;
use ko_protocol::{hex, serde_json, KoResult};
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
        global_cell: &KoContextGlobalCell,
        project_owner: &Script,
        project_lua_code: &Bytes,
    ) -> KoResult<Lua> {
        // initialize project lua code
        let lua = Lua::new();
        lua.load(&project_lua_code.to_vec())
            .exec()
            .map_err(|err| ExecutorError::ErrorLoadProjectLuaCode(err.to_string()))?;

        // prepare global context `KOC`
        let owner = hex::encode(project_owner.calc_script_hash().raw_data());
        let driver = hex::encode(global_cell.lock_script.calc_script_hash().raw_data());
        let global_table = {
            let json_string = String::from_utf8(global_cell.output_data.to_vec())
                .map_err(|_| ExecutorError::InvalidUTF8FormatForGlobalData)?;
            let value: serde_json::Value = serde_json::from_str(&json_string)
                .map_err(|_| ExecutorError::InvalidJsonFormatForGlobalData(json_string))?;
            luac!(lua.to_value(&value))
        };

        let context = luac!(lua.create_table());
        luac!(context.set("owner", owner));
        luac!(context.set("driver", driver));
        luac!(context.set("global", global_table));
        luac!(lua.globals().set("KOC", context));

        Ok(lua)
    }
}

impl Executor for ExecutorImpl {
    fn execute_lua_requests(
        &self,
        global_cell: &mut KoContextGlobalCell,
        project_owner: &Script,
        user_requests: &[KoRequest],
        project_lua_code: &Bytes,
        random_seeds: &[i64; 2],
    ) -> KoResult<Vec<KoExecutedRequest>> {
        let lua = self.prepare_lua_context(global_cell, project_owner, project_lua_code)?;
        let math: Table = luac!(lua.globals().get("math"));
        let randomseed: Function = luac!(math.get("randomseed"));
        luac!(randomseed.call((random_seeds[0], random_seeds[1])));

        // running each user function_call requests
        let personal_outputs =
            helper::parse_requests_to_outputs(&lua, project_owner, global_cell, user_requests)?;

        // make final global json string
        global_cell.output_data = {
            let msg: Table = luac!(lua.globals().get("KOC"));
            let global_table: Table = luac!(msg.get("global"));
            let data = serde_json::to_string(&global_table).expect("execute global");
            Bytes::from(data.as_bytes().to_vec())
        };

        // collect results to make execute receipt
        Ok(personal_outputs)
    }

    fn estimate_payment_ckb(
        &self,
        global_cell: &KoContextGlobalCell,
        project_owner: &Script,
        request: KoRequest,
        project_lua_code: &Bytes,
    ) -> KoResult<u64> {
        let lua = self.prepare_lua_context(global_cell, project_owner, project_lua_code)?;
        let context: Table = luac!(lua.globals().get("KOC"));

        // prepare payment ckb catcher
        let payment_ckb = Rc::new(RefCell::new(0u64));
        let payment = payment_ckb.clone();
        let ckb_deposit = luac!(lua.create_function(move |_, ckb: f64| {
            *payment.borrow_mut() = (ckb * 100_000_000.0) as u64;
            Ok(true)
        }));

        // prepare global ckb poller
        let avaliable_ckb = global_cell.capacity - global_cell.occupied_capacity;
        let ckb_withdraw = luac!(lua.create_function(move |_, ckb: f64| {
            let withdraw_ckb = (ckb * 100_000_000.0) as u64;
            Ok(avaliable_ckb >= withdraw_ckb)
        }));

        // inject functions
        luac!(context.set("ckb_deposit", ckb_deposit));
        luac!(context.set("ckb_withdraw", ckb_withdraw));
        luac!(lua.globals().set("KOC", context));

        // run request and trigger ckb_cost function if it exists
        let mut global_driver = global_cell.lock_script.clone();
        helper::run_request(&lua, project_owner, &mut global_driver, &request, 0)?;

        // get payment ckb cost
        Ok(payment_ckb.take())
    }
}
