use std::cell::RefCell;
use std::rc::Rc;

use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::packed::Script;
use ko_protocol::derive_more::Constructor;
use ko_protocol::traits::Executor;
use ko_protocol::types::assembler::{KoCellOutput, KoRequest};
use ko_protocol::types::context::KoContextGlobalCell;
use ko_protocol::{hex, serde_json, KoResult};
use mlua::{Lua, LuaSerdeExt, Table};

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

        let preload = [
            27u8, 76, 117, 97, 84, 0, 25, 147, 13, 10, 26, 10, 4, 8, 8, 120, 86, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 40, 119, 64, 1, 128, 128, 128, 0, 1, 2, 134, 81, 0, 0, 0, 79, 0, 0, 0,
            15, 0, 0, 0, 79, 128, 0, 0, 15, 0, 1, 0, 70, 0, 1, 1, 130, 4, 144, 95, 99, 111, 109,
            112, 97, 114, 101, 95, 116, 97, 98, 108, 101, 115, 4, 139, 95, 100, 101, 101, 112, 95,
            99, 111, 112, 121, 129, 1, 0, 0, 130, 128, 129, 145, 2, 0, 11, 174, 11, 1, 0, 0, 128,
            1, 0, 0, 68, 1, 2, 5, 75, 1, 12, 0, 11, 4, 0, 1, 128, 4, 7, 0, 68, 4, 2, 2, 60, 4, 2,
            0, 184, 6, 0, 128, 11, 4, 0, 1, 140, 4, 1, 6, 68, 4, 2, 2, 60, 4, 2, 0, 184, 2, 0, 128,
            11, 4, 0, 3, 128, 4, 7, 0, 12, 5, 1, 6, 68, 4, 3, 2, 60, 4, 4, 0, 184, 3, 0, 128, 5, 4,
            0, 0, 70, 132, 2, 0, 56, 2, 0, 128, 12, 4, 1, 6, 185, 131, 8, 0, 184, 0, 0, 128, 5, 4,
            0, 0, 70, 132, 2, 0, 76, 1, 0, 2, 77, 1, 13, 0, 54, 1, 0, 0, 11, 1, 0, 0, 128, 1, 1, 0,
            68, 1, 2, 5, 75, 129, 2, 0, 12, 4, 0, 6, 60, 4, 5, 0, 184, 0, 0, 128, 5, 4, 0, 0, 70,
            132, 2, 0, 76, 1, 0, 2, 77, 129, 3, 0, 54, 1, 0, 0, 7, 1, 0, 0, 70, 129, 2, 0, 70, 129,
            1, 0, 134, 4, 134, 112, 97, 105, 114, 115, 4, 133, 116, 121, 112, 101, 4, 134, 116, 97,
            98, 108, 101, 4, 144, 95, 99, 111, 109, 112, 97, 114, 101, 95, 116, 97, 98, 108, 101,
            115, 1, 0, 129, 0, 0, 0, 128, 128, 128, 128, 128, 128, 146, 155, 1, 0, 10, 149, 139, 0,
            0, 0, 0, 1, 0, 0, 196, 0, 2, 2, 188, 128, 1, 0, 56, 0, 0, 128, 70, 128, 2, 0, 147, 0,
            0, 0, 82, 0, 0, 0, 11, 1, 0, 2, 128, 1, 0, 0, 68, 1, 2, 5, 75, 1, 2, 0, 11, 4, 0, 3,
            128, 4, 7, 0, 68, 4, 2, 2, 144, 0, 6, 8, 76, 1, 0, 2, 77, 1, 3, 0, 54, 1, 0, 0, 198,
            128, 2, 0, 70, 129, 1, 0, 132, 4, 133, 116, 121, 112, 101, 4, 134, 116, 97, 98, 108,
            101, 4, 134, 112, 97, 105, 114, 115, 4, 139, 95, 100, 101, 101, 112, 95, 99, 111, 112,
            121, 129, 0, 0, 0, 128, 128, 128, 128, 128, 128, 128, 128, 128,
        ];
        luac!(lua.load(&preload[..]).exec());

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
    ) -> KoResult<Vec<KoResult<KoCellOutput>>> {
        let lua = self.prepare_lua_context(global_cell, project_owner, project_lua_code)?;

        // applying random seeds
        helper::apply_randomseed(&lua, random_seeds)?;

        // running each user function_call requests
        let personal_outputs =
            helper::parse_requests_to_outputs(&lua, project_owner, global_cell, user_requests)?;

        // make final global json string
        global_cell.output_data = {
            let context: Table = luac!(lua.globals().get("KOC"));
            let global_table: Table = luac!(context.get("global"));
            let data = serde_json::to_string(&global_table).expect("execute global");
            println!("global_data = {}", data);
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
