use std::time::Duration;

use ko_context_assembler::AssemblerImpl;
use ko_context_driver::DriverImpl;
use ko_context_executor::ExecutorImpl;
use ko_protocol::ckb_types::packed::{CellDep, OutPoint, Script};
use ko_protocol::ckb_types::prelude::Unpack;
use ko_protocol::ckb_types::{bytes::Bytes, H256};
use ko_protocol::secp256k1::SecretKey;
use ko_protocol::tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use ko_protocol::traits::{Assembler, CkbClient, Context, Driver, Executor};
use ko_protocol::types::assembler::{KoCellOutput, KoRequest};
use ko_protocol::types::context::KoContextRpcEcho;
use ko_protocol::{async_trait, tokio, KoResult, ProjectDeps};

#[cfg(test)]
mod tests;

#[derive(Default)]
struct ProjectContext {
    pub contract_code: Bytes,
    pub global_json_data: Bytes,
    pub owner_lockhash: H256,
}

pub struct ContextImpl<C: CkbClient> {
    pub assembler: AssemblerImpl<C>,
    pub executor: ExecutorImpl,
    pub driver: DriverImpl<C>,

    drive_interval: Duration,
    max_reqeusts_count: u8,
    block_confirms_count: u8,

    invalid_outpoints: Vec<OutPoint>,
    project_context: ProjectContext,
    rpc_receiver: UnboundedReceiver<KoContextRpcEcho>,
}

impl<C: CkbClient> ContextImpl<C> {
    pub fn new(
        rpc_client: &C,
        privkey: &SecretKey,
        project_deps: &ProjectDeps,
    ) -> (ContextImpl<C>, UnboundedSender<KoContextRpcEcho>) {
        let (sender, receiver) = unbounded_channel();
        let context = ContextImpl {
            assembler: AssemblerImpl::new(rpc_client, project_deps),
            executor: ExecutorImpl::new(),
            driver: DriverImpl::new(rpc_client, privkey),
            drive_interval: Duration::from_secs(3),
            max_reqeusts_count: 20,
            block_confirms_count: 3,
            invalid_outpoints: Vec::new(),
            project_context: ProjectContext::default(),
            rpc_receiver: receiver,
        };
        (context, sender)
    }

    pub fn set_drive_interval(&mut self, interval: u8) {
        self.drive_interval = Duration::from_secs(interval as u64);
    }

    pub fn set_max_requests_count(&mut self, requests_count: u8) {
        self.max_reqeusts_count = requests_count;
    }

    pub fn set_confirms_count(&mut self, confirms: u8) {
        self.block_confirms_count = confirms;
    }

    pub async fn start_drive_loop(&mut self, project_cell_deps: &[CellDep]) -> KoResult<()> {
        let project_dep = self
            .assembler
            .prepare_ko_transaction_project_celldep()
            .await?;
        let mut transaction_deps = project_cell_deps.to_vec();
        transaction_deps.insert(0, project_dep.cell_dep);

        let original_interval = self.drive_interval;
        self.drive_interval = Duration::ZERO;
        self.project_context.contract_code = project_dep.lua_code.clone();

        println!("[INFO] knside-out drive server started, enter drive loop");
        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.drive_interval) => {
                    if let Some(hash) = self.drive(&project_dep.lua_code, &transaction_deps).await? {
                        println!(
                            "[INFO] send knside-out tansaction({}), wait `COMMITTED` status...",
                            hash
                        );
                        self.driver
                            .wait_ko_transaction_committed(
                                &hash,
                                &self.drive_interval,
                                self.block_confirms_count,
                            )
                            .await?;
                        self.drive_interval = Duration::ZERO;
                    } else {
                        self.drive_interval = original_interval;
                    }
                }

                Some(echo) = self.rpc_receiver.recv() => match echo {
                    KoContextRpcEcho::EstimatePaymentCkb(
                        ((sender, method_call, previous_json_data, recipient), response)
                    ) => {
                        let payment_ckb = self.estimate_payment_ckb(
                            &sender,
                            &method_call,
                            &previous_json_data,
                            &recipient
                        )?;
                        response.send(payment_ckb).expect("EstimatePaymentCkb channel");
                    }
                }
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
        let project_owner = receipt.global_lockscript.calc_script_hash().unpack();
        let result = self.executor.execute_lua_requests(
            &receipt.global_json_data,
            &project_owner,
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

        // if only have global_cell exist, skip this try
        let inputs = inputs.into_iter().flatten().collect::<Vec<_>>();
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

        // record last running context
        self.project_context.global_json_data = receipt.global_json_data.clone();
        self.project_context.owner_lockhash = project_owner;

        Ok(Some(hash))
    }
}

#[async_trait]
impl<C: CkbClient> Context for ContextImpl<C> {
    fn get_project_id(&self) -> H256 {
        self.assembler.get_project_id()
    }

    fn estimate_payment_ckb(
        &self,
        sender: &Script,
        method_call: &str,
        previous_json_data: &str,
        recipient: &Option<Script>,
    ) -> KoResult<u64> {
        let request = KoRequest::new(
            Bytes::from(previous_json_data.as_bytes().to_vec()),
            Bytes::from(method_call.as_bytes().to_vec()),
            sender.clone(),
            recipient.clone(),
            0,
            0,
        );
        self.executor.estimate_payment_ckb(
            &self.project_context.global_json_data,
            &self.project_context.owner_lockhash,
            request,
            &self.project_context.contract_code,
        )
    }

    async fn run(mut self, project_cell_deps: &[CellDep]) {
        loop {
            // handle exception operation
            let ctrl_c_handler = async {
                #[cfg(windows)]
                let _ = tokio::signal::ctrl_c().await;
                #[cfg(unix)]
                {
                    use tokio::signal::unix;
                    let mut sigtun_int = unix::signal(unix::SignalKind::interrupt()).unwrap();
                    let mut sigtun_term = unix::signal(unix::SignalKind::terminate()).unwrap();
                    tokio::select! {
                        _ = sigtun_int.recv() => {}
                        _ = sigtun_term.recv() => {}
                    };
                }
            };

            // enter drive loop, will stop when any type of error throwed out
            tokio::select! {
                _ = ctrl_c_handler => {
                    println!("<Ctrl-C> is on call, quit knside-out drive loop");
                    break;
                },
                Err(error) = self.start_drive_loop(project_cell_deps) => {
                    println!("[ERROR] {}", error);
                }
            }
        }
    }
}
