use std::time::Duration;

use ko_core_assembler::AssemblerImpl;
use ko_core_driver::DriverImpl;
use ko_core_executor::ExecutorImpl;
use ko_protocol::ckb_types::packed::{CellDep, OutPoint};
use ko_protocol::ckb_types::prelude::Unpack;
use ko_protocol::ckb_types::{bytes::Bytes, H256};
use ko_protocol::secp256k1::SecretKey;
use ko_protocol::traits::{Assembler, CkbClient, Driver, Executor};
use ko_protocol::types::assembler::KoCellOutput;
use ko_protocol::types::config::KoCellDep;
use ko_protocol::{tokio, KoResult, ProjectDeps};

#[cfg(test)]
mod tests;

pub struct Context<C: CkbClient> {
    pub assembler: AssemblerImpl<C>,
    pub executor: ExecutorImpl,
    pub driver: DriverImpl<C>,

    drive_interval: Duration,
    max_reqeusts_count: u8,

    invalid_outpoints: Vec<OutPoint>,
}

impl<C: CkbClient> Context<C> {
    pub fn new(rpc_client: &C, privkey: &SecretKey, project_deps: &ProjectDeps) -> Context<C> {
        Context {
            assembler: AssemblerImpl::new(rpc_client, project_deps),
            executor: ExecutorImpl::new(),
            driver: DriverImpl::new(rpc_client, privkey),
            drive_interval: Duration::from_secs(3),
            max_reqeusts_count: 20,
            invalid_outpoints: Vec::new(),
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

    pub async fn start(mut self, project_cell_deps: &[KoCellDep]) -> KoResult<()> {
        let project_dep = self
            .assembler
            .prepare_ko_transaction_project_celldep()
            .await?;
        let mut transaction_deps = self
            .driver
            .prepare_ko_transaction_normal_celldeps(project_cell_deps)
            .await?;
        transaction_deps.insert(0, project_dep.cell_dep);

        println!("[INFO] knside-out drive server started, enter drive loop");
        loop {
            let hash = self.drive(&project_dep.lua_code, &transaction_deps).await?;
            if let Some(hash) = hash {
                println!(
                    "[INFO] send knside-out tansaction({}), wait commit...",
                    hash
                );
                self.driver
                    .wait_ko_transaction_committed(&hash, &self.drive_interval)
                    .await?;
            } else {
                tokio::time::sleep(self.drive_interval).await;
            }
        }
    }

    pub async fn drive(
        &mut self,
        project_lua_code: &Bytes,
        project_cell_deps: &[CellDep],
    ) -> KoResult<Option<H256>> {
        // assemble knside-out transaction
        let (tx, receipt) = self
            .assembler
            .generate_ko_transaction_with_inputs_and_celldeps(
                self.max_reqeusts_count,
                project_cell_deps,
                &self.invalid_outpoints,
            )
            .await?;
        if receipt.requests.is_empty() {
            return Ok(None);
        }
        let result = self.executor.execute_lua_requests(
            &receipt.global_json_data,
            &receipt.global_lockscript.calc_script_hash().unpack(),
            &receipt.requests,
            project_lua_code,
        )?;
        let mut cell_outputs = vec![KoCellOutput::new(
            Some(result.global_json_data),
            receipt.global_lockscript,
        )];
        // trim unworkable requests from transaction inputs
        let mut inputs = tx.inputs().into_iter().map(Some).collect::<Vec<_>>();
        let mut total_inputs_capacity = receipt.global_ckb;
        result
            .personal_outputs
            .into_iter()
            .enumerate()
            .for_each(|(i, output)| match output {
                Ok((data, lock_script)) => {
                    cell_outputs.push(KoCellOutput::new(data, lock_script));
                    total_inputs_capacity += receipt.requests[i].ckb;
                }
                Err(err) => {
                    // because the frist input is global_cell, so the offset is 1
                    let outpoint = inputs[i + 1].as_ref().unwrap().previous_output();
                    self.invalid_outpoints.push(outpoint);
                    inputs[i + 1] = None;
                    println!("[ERROR] {} [SKIP]", err);
                }
            });
        let inputs = inputs.into_iter().flatten().collect::<Vec<_>>();
        // if only have global_cell exist, skip this try
        if inputs.len() == 1 {
            return Ok(None);
        }
        let tx = tx.as_advanced_builder().set_inputs(inputs).build();
        let tx = self
            .assembler
            .fill_ko_transaction_with_outputs(tx, &cell_outputs, total_inputs_capacity)
            .await?;
        let signature = self.driver.sign_ko_transaction(&tx);
        let tx = self
            .assembler
            .complete_ko_transaction_with_signature(tx, signature);
        let hash = self.driver.send_ko_transaction(tx).await?;
        Ok(Some(hash))
    }
}
