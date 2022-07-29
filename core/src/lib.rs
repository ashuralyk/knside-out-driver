use std::time::Duration;

use ko_protocol::ckb_types::packed::CellDep;
use ko_protocol::ckb_types::prelude::Unpack;
use ko_protocol::ckb_types::{H256, bytes::Bytes};
use ko_protocol::tokio;
use ko_protocol::traits::{Assembler, Driver, Executor};
use ko_protocol::types::assembler::KoCellOutput;
use ko_protocol::types::config::KoCellDep;
use ko_protocol::KoResult;

#[cfg(test)]
mod tests;

pub struct Context<A, E, D>
where
    A: Assembler,
    E: Executor,
    D: Driver,
{
    ko_assembler: A,
    ko_executor: E,
    ko_driver: D,

    drive_interval: Duration,
    max_reqeusts_count: u8,
}

impl<A, E, D> Context<A, E, D>
where
    A: Assembler,
    E: Executor,
    D: Driver,
{
    pub fn new(assembler: A, executor: E, driver: D) -> Context<A, E, D> {
        Context {
            ko_assembler: assembler,
            ko_executor: executor,
            ko_driver: driver,
            drive_interval: Duration::from_secs(3),
            max_reqeusts_count: 20,
        }
    }

    pub fn set_drive_interval(mut self, interval: Duration) -> Self {
        self.drive_interval = interval;
        self
    }

    pub fn set_max_requests_count(mut self, requests_count: u8) -> Self {
        self.max_reqeusts_count = requests_count;
        self
    }

    pub async fn start(
        self,
        project_cell_deps: &[KoCellDep],
    ) -> KoResult<()> {
        let project_dep = self
            .ko_assembler
            .prepare_ko_transaction_project_celldep()
            .await?;
        let transaction_deps = self
            .ko_driver
            .prepare_ko_transaction_normal_celldeps(project_cell_deps)
            .await?;
        let mut interval = tokio::time::interval(self.drive_interval);

        loop {
            let (hash, _) = tokio::join!(
                self.drive(&project_dep.lua_code, &transaction_deps),
                interval.tick()
            );
            if let Some(hash) = hash? {
                println!("[Core] knside-out tansaction hash = {}", hash);
            }
        }
    }

    pub async fn drive(
        &self,
        project_lua_code: &Bytes,
        project_cell_deps: &[CellDep],
    ) -> KoResult<Option<H256>> {
        let (tx, receipt) = self
            .ko_assembler
            .generate_ko_transaction_with_inputs_and_celldeps(
                self.max_reqeusts_count,
                &project_cell_deps,
            )
            .await?;
        if receipt.requests.is_empty() {
            return Ok(None);
        }
        let result = self.ko_executor.execute_lua_requests(
            &receipt.global_json_data,
            &receipt.global_lockscript.calc_script_hash().unpack(),
            &receipt.requests,
            &project_lua_code,
        )?;
        let mut cell_outputs = vec![KoCellOutput::new(
            receipt.global_json_data,
            receipt.global_lockscript,
            0,
        )];
        // assemble transaction outputs import
        receipt
            .requests
            .into_iter()
            .enumerate()
            .for_each(|(i, request)| {
                cell_outputs.push(KoCellOutput::new(
                    result.outputs_json_data[i].clone(),
                    request.lock_script,
                    result.required_payments[i],
                ));
            });
        let tx = self
            .ko_assembler
            .fill_ko_transaction_with_outputs(tx, &cell_outputs, receipt.total_inputs_capacity)
            .await?;
        let signature = self.ko_driver.sign_ko_transaction(&tx);
        let tx = self
            .ko_assembler
            .complete_ko_transaction_with_signature(tx, signature);
        let hash = self.ko_driver.send_ko_transaction(tx).await?;
        Ok(Some(hash))
    }
}
