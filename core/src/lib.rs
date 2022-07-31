use std::time::Duration;

use ko_protocol::ckb_types::packed::CellDep;
use ko_protocol::ckb_types::prelude::Unpack;
use ko_protocol::ckb_types::{bytes::Bytes, H256};
use ko_protocol::traits::{Assembler, Driver, Executor};
use ko_protocol::types::assembler::KoCellOutput;
use ko_protocol::types::config::KoCellDep;
use ko_protocol::{tokio, KoResult};

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

    pub async fn start(self, project_cell_deps: &[KoCellDep]) -> KoResult<()> {
        let project_dep = self
            .ko_assembler
            .prepare_ko_transaction_project_celldep()
            .await?;
        let mut transaction_deps = self
            .ko_driver
            .prepare_ko_transaction_normal_celldeps(project_cell_deps)
            .await?;
        transaction_deps.insert(0, project_dep.cell_dep);

        println!("[INFO] knside-out drive server started, enter drive loop");
        loop {
            let hash = self.drive(&project_dep.lua_code, &transaction_deps).await?;
            if let Some(hash) = hash {
                println!(
                    "[INFO] send knside-out tansaction({}), wait for committed...",
                    hash
                );
                self.ko_driver
                    .wait_ko_transaction_committed(&hash, &self.drive_interval)
                    .await?;
            } else {
                tokio::time::sleep(self.drive_interval).await;
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
                project_cell_deps,
            )
            .await?;
        if receipt.requests.is_empty() {
            return Ok(None);
        }
        let result = self.ko_executor.execute_lua_requests(
            &receipt.global_json_data,
            &receipt.global_lockscript.calc_script_hash().unpack(),
            &receipt.requests,
            project_lua_code,
        )?;
        let mut cell_outputs = vec![KoCellOutput::new(
            Some(result.global_json_data),
            receipt.global_lockscript,
        )];
        // assemble transaction outputs import
        for i in 0..receipt.requests.len() {
            let (data, lock_script) = result.personal_outputs[i].clone();
            cell_outputs.push(KoCellOutput::new(data, lock_script));
        }
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
