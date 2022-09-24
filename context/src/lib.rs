use std::collections::HashMap;
use std::time::Duration;

use ko_context_assembler::AssemblerImpl;
use ko_context_driver::DriverImpl;
use ko_context_executor::ExecutorImpl;
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::packed::Script;
use ko_protocol::ckb_types::prelude::Unpack;
use ko_protocol::secp256k1::SecretKey;
use ko_protocol::tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use ko_protocol::tokio::sync::Mutex;
use ko_protocol::tokio::task::JoinHandle;
use ko_protocol::traits::{Assembler, CkbClient, ContextRpc, Driver, Executor};
use ko_protocol::types::assembler::{KoCellOutput, KoProject, KoRequest};
use ko_protocol::types::config::KoDriveConfig;
use ko_protocol::types::context::{KoContextGlobalCell, KoContextRpcEcho};
use ko_protocol::{async_trait, lazy_static, log, tokio, KoResult, ProjectDeps, H256};

#[cfg(test)]
mod tests;

#[derive(Default)]
struct ProjectContext {
    pub contract_code: Bytes,
    pub project_owner: Script,
    pub global_cell: KoContextGlobalCell,
}

pub struct ContextImpl<C: CkbClient> {
    pub assembler: AssemblerImpl<C>,
    pub executor: ExecutorImpl,
    pub driver: DriverImpl<C>,

    drive_interval: Duration,
    idle_duration: Duration,
    config: KoDriveConfig,

    project_context: ProjectContext,
    rpc_receiver: UnboundedReceiver<KoContextRpcEcho>,
    listening_requests: HashMap<H256, UnboundedSender<KoResult<H256>>>,
}

impl<C: CkbClient> ContextImpl<C> {
    pub fn new(
        rpc_client: &C,
        privkey: &SecretKey,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
        config: &KoDriveConfig,
    ) -> (ContextImpl<C>, UnboundedSender<KoContextRpcEcho>) {
        let (sender, receiver) = unbounded_channel();
        let context = ContextImpl {
            assembler: AssemblerImpl::new(rpc_client, project_type_args, project_deps),
            executor: ExecutorImpl::new(),
            driver: DriverImpl::new(rpc_client, privkey),
            drive_interval: Duration::ZERO,
            idle_duration: Duration::ZERO,
            config: config.clone(),
            project_context: ProjectContext::default(),
            rpc_receiver: receiver,
            listening_requests: HashMap::new(),
        };
        (context, sender)
    }

    async fn start_drive_loop(&mut self) -> KoResult<()> {
        let project_dep = self.assembler.prepare_transaction_project_celldep().await?;
        self.project_context.contract_code = project_dep.lua_code.clone();
        self.project_context.project_owner = project_dep.contract_owner.clone();
        self.project_context.global_cell = self.assembler.get_project_global_cell().await?;

        log::info!(
            "[{}] knside-out drive server started new drive loop",
            self.assembler.get_project_args()
        );
        let drive_interval = Duration::from_secs(self.config.drive_interval_sec as u64);
        let max_idle_duration = Duration::from_secs(self.config.kickout_duration_sec);
        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.drive_interval) => {
                    if let Some(hash) = self.drive(&project_dep).await? {
                        log::info!(
                            "[{}] transaction #{} confirmed",
                            self.assembler.get_project_args(),
                            hash
                        );
                        self.idle_duration = Duration::ZERO;
                        self.drive_interval = Duration::ZERO;
                    } else {
                        self.drive_interval = drive_interval;
                        self.idle_duration += drive_interval;
                        if self.idle_duration > max_idle_duration {
                            break;
                        }
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
                        );
                        response.send(payment_ckb).expect("EstimatePaymentCkb channel");
                    },
                    KoContextRpcEcho::ListenRequestCommitted((hash, response))=> {
                        self.listen_request_committed(&hash, response);
                    }
                }
            }
        }
        Ok(())
    }

    pub(self) async fn drive(&mut self, project_dep: &KoProject) -> KoResult<Option<H256>> {
        // assemble knside-out transaction
        let (tx, mut receipt) = self
            .assembler
            .generate_transaction_with_inputs_and_celldeps(
                self.config.max_reqeusts_count,
                &project_dep.cell_dep,
            )
            .await?;
        if receipt.requests.is_empty() {
            return Ok(None);
        }
        let personal_outputs = self.executor.execute_lua_requests(
            &mut receipt.global_cell,
            &project_dep.contract_owner,
            &receipt.requests,
            &project_dep.lua_code,
            &receipt.random_seeds,
        )?;
        let mut cell_outputs = vec![KoCellOutput::new(
            Some(receipt.global_cell.output_data.clone()),
            receipt.global_cell.lock_script.clone(),
            receipt.global_cell.capacity,
        )];

        // trim unworkable requests from transaction inputs
        let mut request_hashes = tx
            .inputs()
            .into_iter()
            .skip(1)
            .map(|input| (input.previous_output().tx_hash().unpack(), None))
            .collect::<Vec<(H256, _)>>();
        let mut total_inputs_capacity = receipt.global_cell.capacity;
        personal_outputs
            .into_iter()
            .enumerate()
            .for_each(|(i, output)| match output {
                Ok(output_assemble) => {
                    cell_outputs.push(output_assemble);
                    total_inputs_capacity += receipt.requests[i].capacity;
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
                        request.capacity,
                    ));
                    total_inputs_capacity += receipt.requests[i].capacity;
                    request_hashes[i].1 = Some(err);
                }
            });

        // complete transaction
        let tx = self
            .assembler
            .fill_transaction_with_outputs(tx, &cell_outputs, total_inputs_capacity, 100_000_000)
            .await?;
        let signature = self.driver.sign_transaction(&tx);
        let tx = self
            .assembler
            .complete_transaction_with_signature(tx, signature);
        let hash = self.driver.send_transaction(tx).await?;

        // record last running context
        self.project_context.global_cell = receipt.global_cell;

        // wait transaction has been confirmed for enough confirmations
        self.driver
            .wait_transaction_committed(
                &hash,
                &self.drive_interval,
                self.config.block_confirms_count,
            )
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

    pub fn estimate_payment_ckb(
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
            &self.project_context.global_cell,
            &self.project_context.project_owner,
            request,
            &self.project_context.contract_code,
        )
    }

    pub fn listen_request_committed(
        &mut self,
        request_hash: &H256,
        sender: UnboundedSender<KoResult<H256>>,
    ) {
        self.listening_requests.insert(request_hash.clone(), sender);
    }

    pub async fn run(mut self) {
        loop {
            if let Err(error) = self.start_drive_loop().await {
                log::error!("[{}] {}", self.assembler.get_project_args(), error);
            }
            tokio::time::sleep(self.drive_interval).await;
        }
    }
}

type Context = (JoinHandle<()>, UnboundedSender<KoContextRpcEcho>);
lazy_static! {
    static ref CONTEXT_POOL: Mutex<HashMap<H256, Context>> = Mutex::new(HashMap::new());
}

pub struct ContextMgr<C: CkbClient> {
    rpc_client: C,
    private_key: SecretKey,
    project_deps: ProjectDeps,
    driver_config: KoDriveConfig,
}

impl<C: CkbClient + 'static> ContextMgr<C> {
    pub fn new(
        rpc_client: &C,
        private_key: &SecretKey,
        project_deps: &ProjectDeps,
        driver_config: &KoDriveConfig,
    ) -> Self {
        ContextMgr {
            rpc_client: rpc_client.clone(),
            private_key: *private_key,
            project_deps: project_deps.clone(),
            driver_config: driver_config.clone(),
        }
    }

    pub async fn recover_contexts(&mut self, project_type_args_list: Vec<(H256, bool)>) {
        for (hash, running) in project_type_args_list {
            if running {
                self.start_project_driver(&hash).await;
            } else {
                let (sender, _) = unbounded_channel();
                CONTEXT_POOL
                    .lock()
                    .await
                    .insert(hash, (tokio::spawn(async {}), sender));
            }
        }
    }

    pub async fn dump_contexts_status() -> Vec<(H256, bool)> {
        CONTEXT_POOL
            .lock()
            .await
            .iter()
            .map(|(hash, context)| (hash.clone(), !context.0.is_finished()))
            .collect()
    }
}

#[async_trait]
impl<C: CkbClient + 'static> ContextRpc for ContextMgr<C> {
    async fn start_project_driver(&mut self, project_type_args: &H256) -> bool {
        if let Some((ctx, _)) = CONTEXT_POOL.lock().await.get(project_type_args) {
            if !ctx.is_finished() {
                return false;
            }
        }
        let (ctx, rpc) = ContextImpl::new(
            &self.rpc_client,
            &self.private_key,
            project_type_args,
            &self.project_deps,
            &self.driver_config,
        );
        CONTEXT_POOL
            .lock()
            .await
            .insert(project_type_args.clone(), (tokio::spawn(ctx.run()), rpc));
        true
    }

    async fn estimate_payment_ckb(
        &self,
        project_type_args: &H256,
        sender: &Script,
        method_call: &str,
        previous_json_data: &str,
        recipient: &Option<Script>,
        response: UnboundedSender<KoResult<u64>>,
    ) -> bool {
        if let Some((ctx, rpc_sender)) = CONTEXT_POOL.lock().await.get(project_type_args) {
            if !ctx.is_finished() {
                let params = KoContextRpcEcho::EstimatePaymentCkb((
                    (
                        sender.clone(),
                        method_call.into(),
                        previous_json_data.into(),
                        recipient.clone(),
                    ),
                    response,
                ));
                rpc_sender.send(params).unwrap();
                return true;
            }
        }
        false
    }

    async fn listen_request_committed(
        &self,
        project_type_args: &H256,
        request_hash: &H256,
        response: UnboundedSender<KoResult<H256>>,
    ) -> bool {
        if let Some((ctx, rpc_sender)) = CONTEXT_POOL.lock().await.get(project_type_args) {
            if ctx.is_finished() {
                let params =
                    KoContextRpcEcho::ListenRequestCommitted((request_hash.clone(), response));
                rpc_sender.send(params).unwrap();
                return true;
            }
        }
        false
    }
}
