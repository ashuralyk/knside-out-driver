use std::collections::HashMap;
use std::time::Duration;

use ko_context_assembler::AssemblerImpl;
use ko_context_driver::DriverImpl;
use ko_context_executor::ExecutorImpl;
use ko_protocol::ckb_types::packed::{CellDep, Script};
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

    project_context: ProjectContext,
    rpc_receiver: UnboundedReceiver<KoContextRpcEcho>,
    listening_requests: HashMap<H256, UnboundedSender<KoResult<H256>>>,
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
            project_context: ProjectContext::default(),
            rpc_receiver: receiver,
            listening_requests: HashMap::new(),
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
        let project_dep = self.assembler.prepare_transaction_project_celldep().await?;
        let mut transaction_deps = project_cell_deps.to_vec();
        transaction_deps.insert(0, project_dep.cell_dep);

        let (project_owner, global_data) = self.assembler.get_project_owner_and_global().await?;
        self.project_context.contract_code = project_dep.lua_code.clone();
        self.project_context.owner_lockhash = project_owner;
        self.project_context.global_json_data = global_data;

        println!("[INFO] knside-out drive server started, enter drive loop");
        let startup_interval = self.drive_interval;
        self.drive_interval = Duration::ZERO;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.drive_interval) => {
                    if let Some(hash) = self.drive(&project_dep.lua_code, &transaction_deps).await? {
                        println!("[INFO] transaction #{} confirmed", hash);
                        self.drive_interval = Duration::ZERO;
                    } else {
                        self.drive_interval = startup_interval;
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
                    },
                    KoContextRpcEcho::ListenRequestCommitted((hash, response))=> {
                        self.listen_request_committed(&hash, response);
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
            .generate_transaction_with_inputs_and_celldeps(
                self.max_reqeusts_count,
                project_cell_deps,
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
            &receipt.random_seeds,
        )?;
        let mut cell_outputs = vec![KoCellOutput::new(
            Some(result.global_json_data),
            receipt.global_lockscript,
            0,
        )];

        // trim unworkable requests from transaction inputs
        let mut request_hashes = tx
            .inputs()
            .into_iter()
            .skip(1)
            .map(|input| (input.previous_output().tx_hash().unpack(), None))
            .collect::<Vec<(H256, _)>>();
        let mut total_inputs_capacity = receipt.global_ckb;
        result
            .personal_outputs
            .into_iter()
            .enumerate()
            .for_each(|(i, output)| match output {
                Ok((data, lock_script)) => {
                    cell_outputs.push(KoCellOutput::new(data, lock_script, 0));
                    total_inputs_capacity += receipt.requests[i].ckb;
                }
                Err(err) => {
                    // recover the previous cell before its request operation
                    let request = &receipt.requests[i];
                    let data = {
                        if request.json_data.is_empty() {
                            None
                        } else {
                            Some(request.json_data.clone())
                        }
                    };
                    cell_outputs.push(KoCellOutput::new(
                        data,
                        request.lock_script.clone(),
                        request.payment,
                    ));
                    total_inputs_capacity += receipt.requests[i].ckb;
                    request_hashes[i].1 = Some(err);
                }
            });

        // complete transaction
        let tx = self
            .assembler
            .fill_transaction_with_outputs(tx, &cell_outputs, total_inputs_capacity)
            .await?;
        let signature = self.driver.sign_transaction(&tx);
        let tx = self
            .assembler
            .complete_transaction_with_signature(tx, signature);
        let hash = self.driver.send_transaction(tx).await?;

        // record last running context
        self.project_context.global_json_data = receipt.global_json_data.clone();
        self.project_context.owner_lockhash = project_owner;

        // wait transaction has been confirmed for enough confirmations
        self.driver
            .wait_transaction_committed(&hash, &self.drive_interval, self.block_confirms_count)
            .await?;

        // clear request listening callbacks
        request_hashes
            .into_iter()
            .for_each(|(request_hash, error)| {
                if let Some(callback) = self.listening_requests.remove(&request_hash) {
                    if let Some(err) = error {
                        callback.send(Err(err)).expect("clear callback");
                    } else {
                        callback.send(Ok(request_hash)).expect("clear callback");
                    }
                }
            });

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

    fn listen_request_committed(
        &mut self,
        request_hash: &H256,
        sender: UnboundedSender<KoResult<H256>>,
    ) {
        self.listening_requests.insert(request_hash.clone(), sender);
    }

    async fn run(mut self, project_cell_deps: &[CellDep]) {
        loop {
            if let Err(error) = self.start_drive_loop(project_cell_deps).await {
                println!("[ERROR] {}", error);
            }
            tokio::time::sleep(self.drive_interval).await;
        }
    }
}
