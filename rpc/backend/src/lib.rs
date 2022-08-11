use std::collections::HashMap;
use std::str::FromStr;

use ko_protocol::ckb_jsonrpc_types::{OutputsValidator, TransactionView as JsonTxView};
use ko_protocol::ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey, SearchKeyFilter};
use ko_protocol::ckb_sdk::Address;
use ko_protocol::ckb_types::core::{Capacity, TransactionBuilder, TransactionView};
use ko_protocol::ckb_types::packed::{CellInput, CellOutput, OutPoint, Script, Transaction};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack, Unpack};
use ko_protocol::ckb_types::{bytes::Bytes, H256};
use ko_protocol::serde_json::to_string;
use ko_protocol::tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use ko_protocol::traits::{Backend, CkbClient};
use ko_protocol::types::context::KoContextRpcEcho;
use ko_protocol::types::generated::{mol_deployment, mol_flag_0};
use ko_protocol::{async_trait, mol_flag_1, mol_flag_2, KoResult, ProjectDeps};

#[cfg(test)]
mod tests;

mod error;
mod helper;
use error::BackendError;

pub struct BackendImpl<C: CkbClient> {
    rpc_client: C,
    cached_transactions: HashMap<H256, TransactionView>,
    context_rpc: Option<UnboundedSender<KoContextRpcEcho>>,

    estimate_payment_ckb_sender: UnboundedSender<u64>,
    estimate_payment_ckb_receiver: UnboundedReceiver<u64>,
}

impl<C: CkbClient> BackendImpl<C> {
    pub fn new(rpc_client: &C, context: Option<UnboundedSender<KoContextRpcEcho>>) -> Self {
        let (sender, receiver) = unbounded_channel();
        BackendImpl {
            rpc_client: rpc_client.clone(),
            cached_transactions: HashMap::new(),
            context_rpc: context,
            estimate_payment_ckb_sender: sender,
            estimate_payment_ckb_receiver: receiver,
        }
    }

    pub fn peak_transaction(&self, digest: &H256) -> Option<TransactionView> {
        self.cached_transactions.get(digest).cloned()
    }
}

#[async_trait]
impl<C: CkbClient> Backend for BackendImpl<C> {
    async fn create_project_deploy_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_deps: &ProjectDeps,
    ) -> KoResult<(H256, H256)> {
        // make global of output-data
        let global_data_json = helper::get_global_json_data(&contract)?;
        println!("global_data_json = {}", global_data_json);

        // build mock knside-out transaction outputs and data
        let ckb_address =
            Address::from_str(&address).map_err(|_| BackendError::InvalidAddressFormat(address))?;
        println!("address = {}", ckb_address);
        let secp256k1_script: Script = ckb_address.payload().into();
        let global_type_script = helper::build_knsideout_script(
            &project_deps.project_code_hash,
            mol_flag_0(&[0u8; 32]).as_slice(),
        );
        let deployment = mol_deployment(&contract).as_bytes();
        let mut outputs = vec![
            // project cell
            CellOutput::new_builder()
                .lock(secp256k1_script.clone())
                .type_(helper::build_type_id_script(None, 0))
                .build_exact_capacity(Capacity::bytes(deployment.len()).unwrap())
                .unwrap(),
            // global cell
            CellOutput::new_builder()
                .lock(secp256k1_script.clone())
                .type_(Some(global_type_script).pack())
                .build_exact_capacity(Capacity::bytes(global_data_json.len()).unwrap())
                .unwrap(),
            // change cell
            CellOutput::new_builder()
                .lock(secp256k1_script.clone())
                .build_exact_capacity(Capacity::zero())
                .unwrap(),
        ];
        let outputs_data = vec![
            deployment,
            Bytes::from(global_data_json.as_bytes().to_vec()),
            Bytes::default(),
        ];
        let outputs_capacity = helper::calc_outputs_capacity(&outputs, "1.0");

        // fill knside-out transaction inputs
        let search = SearchKey {
            script: secp256k1_script.into(),
            script_type: ScriptType::Lock,
            filter: None,
        };
        let (inputs, inputs_capacity) =
            helper::fetch_live_cells(&self.rpc_client, search, 0, outputs_capacity).await?;
        println!(
            "inputs_capacity = {}, outputs_capacity = {}",
            inputs_capacity, outputs_capacity
        );
        if inputs_capacity < outputs_capacity {
            return Err(BackendError::InternalTransactionAssembleError.into());
        }

        // rebuild type_id with real input and change
        let project_type_script = helper::build_type_id_script(Some(&inputs[0]), 0)
            .to_opt()
            .unwrap();
        let project_type_args = {
            let args: Bytes = project_type_script.args().unpack();
            H256::from_slice(&args).unwrap()
        };
        let project_type_id = project_type_script.calc_script_hash();
        outputs[0] = outputs[0]
            .clone()
            .as_builder()
            .type_(Some(project_type_script).pack())
            .build();
        let global_type_script = helper::build_knsideout_script(
            &project_deps.project_code_hash,
            mol_flag_0(&project_type_id.unpack().0).as_slice(),
        );
        outputs[1] = outputs[1]
            .clone()
            .as_builder()
            .type_(Some(global_type_script).pack())
            .build();
        let change = inputs_capacity - outputs_capacity;
        outputs[2] = outputs[2]
            .clone()
            .as_builder()
            .build_exact_capacity(Capacity::shannons(change))
            .unwrap();

        // build knside-out transaction
        let tx = TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs)
            .outputs_data(outputs_data.pack())
            .cell_deps(project_deps.project_cell_deps.clone())
            .build();

        // generate transaction digest
        let digest = helper::get_transaction_digest(&tx);
        self.cached_transactions.insert(digest.clone(), tx);

        // generate project type_id args
        Ok((digest, project_type_args))
    }

    async fn create_project_update_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_deps: &ProjectDeps,
    ) -> KoResult<H256> {
        // search existed project deployment cell on CKB
        let project_type_script =
            helper::recover_type_id_script(project_deps.project_type_args.as_bytes());
        let search_key = SearchKey {
            script: project_type_script.into(),
            script_type: ScriptType::Type,
            filter: None,
        };
        let result = self
            .rpc_client
            .fetch_live_cells(search_key, 1, None)
            .await
            .map_err(|err| BackendError::IndexerRpcError(err.to_string()))?;
        if result.objects.is_empty() {
            return Err(BackendError::MissProjectDeploymentCell(
                project_deps.project_type_args.clone(),
            )
            .into());
        }
        let deployment_cell = &result.objects[0];

        // build knside-out transaction outputs
        let secp256k1_script: Script = Address::from_str(&address)
            .map_err(|_| BackendError::InvalidAddressFormat(address))?
            .payload()
            .into();
        let previous_type_script = deployment_cell.output.type_.as_ref().unwrap();
        let deployment = mol_deployment(&contract).as_bytes();
        let mut outputs = vec![
            // new project deployment cell
            CellOutput::new_builder()
                .lock(secp256k1_script.clone())
                .type_(Some(previous_type_script.clone().into()).pack())
                .build_exact_capacity(Capacity::bytes(deployment.len()).unwrap())
                .unwrap(),
            // change cell
            CellOutput::new_builder()
                .lock(secp256k1_script.clone())
                .build_exact_capacity(Capacity::zero())
                .unwrap(),
        ];
        let outputs_data = vec![deployment, Bytes::new()];
        let outputs_capacity = helper::calc_outputs_capacity(&outputs, "1.0");

        // fill kinside-out transaction inputs
        let inputs_capacity: u64 = deployment_cell.output.capacity.into();
        let search = SearchKey {
            script: secp256k1_script.into(),
            script_type: ScriptType::Lock,
            filter: None,
        };
        let (mut inputs, inputs_capacity) =
            helper::fetch_live_cells(&self.rpc_client, search, inputs_capacity, outputs_capacity)
                .await?;
        println!(
            "inputs_&capacity = {}, outputs_capacity = {}",
            inputs_capacity, outputs_capacity
        );
        if inputs_capacity < outputs_capacity {
            return Err(BackendError::InternalTransactionAssembleError.into());
        }
        let old_deployment_cell = CellInput::new_builder()
            .previous_output(deployment_cell.out_point.clone().into())
            .build();
        inputs.insert(0, old_deployment_cell);

        // rebuild change output
        let change = inputs_capacity - outputs_capacity;
        outputs[1] = outputs[1]
            .clone()
            .as_builder()
            .build_exact_capacity(Capacity::shannons(change))
            .unwrap();

        // build knside-out transaction
        let tx = TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs)
            .outputs_data(outputs_data.pack())
            .cell_deps(project_deps.project_cell_deps.clone())
            .build();

        // generate transaction digest
        let digest = helper::get_transaction_digest(&tx);
        self.cached_transactions.insert(digest.clone(), tx);

        Ok(digest)
    }

    async fn create_project_request_digest(
        &mut self,
        address: String,
        recipient: Option<String>,
        previous_cell: Option<OutPoint>,
        function_call: String,
        project_deps: &ProjectDeps,
    ) -> KoResult<(H256, u64)> {
        // build neccessary scripts
        let project_type_id =
            helper::recover_type_id_script(project_deps.project_type_args.as_bytes())
                .calc_script_hash()
                .unpack();
        let secp256k1_script: Script = Address::from_str(&address)
            .map_err(|_| BackendError::InvalidAddressFormat(address))?
            .payload()
            .into();
        let recipient_secp256k1_script = {
            if let Some(recipient) = recipient {
                let script: Script = Address::from_str(&recipient)
                    .map_err(|_| BackendError::InvalidAddressFormat(recipient))?
                    .payload()
                    .into();
                Some(script)
            } else {
                None
            }
        };
        let personal_args = mol_flag_1(&project_type_id.0);
        let personal_script =
            helper::build_knsideout_script(&project_deps.project_code_hash, &personal_args);

        // check previous cell
        let mut previous_json_data = String::new();
        let mut inputs_capacity = 0u64;
        let mut inputs = vec![];
        if let Some(outpoint) = previous_cell {
            let tx: Transaction = self
                .rpc_client
                .get_transaction(&outpoint.tx_hash().unpack())
                .await
                .map_err(|err| BackendError::CkbRpcError(err.to_string()))?
                .unwrap()
                .transaction
                .unwrap()
                .inner
                .into();
            let tx = tx.into_view();
            let index: u32 = outpoint.index().unpack();
            let cell = tx.output_with_data(index as usize);
            if let Some((cell, data)) = cell {
                previous_json_data = String::from_utf8(data.to_vec())
                    .map_err(|_| BackendError::InvalidPrevousCell)?;
                inputs_capacity = cell.capacity().unpack();
                inputs.push(CellInput::new_builder().previous_output(outpoint).build());
            } else {
                return Err(BackendError::InvalidPrevousCell.into());
            }
        }

        // request payment ckb of this call
        let payment_ckb = {
            if let Some(rpc) = &self.context_rpc {
                rpc.send(KoContextRpcEcho::EstimatePaymentCkb((
                    (
                        secp256k1_script.clone(),
                        function_call.clone(),
                        previous_json_data.clone(),
                        recipient_secp256k1_script.clone(),
                    ),
                    self.estimate_payment_ckb_sender.clone(),
                )))
                .expect("EstimatePaymentCkb channel request");
                self.estimate_payment_ckb_receiver.recv().await.unwrap()
            } else {
                0u64
            }
        };

        // build reqeust transaction outputs
        let request_args = mol_flag_2(
            &function_call,
            secp256k1_script.as_slice(),
            recipient_secp256k1_script.map(|v| v.as_bytes()),
        );
        let request_script =
            helper::build_knsideout_script(&project_deps.project_code_hash, &request_args);
        let request_capacity =
            Capacity::bytes(previous_json_data.len()).unwrap().as_u64() + payment_ckb;
        let mut outputs = vec![
            // reqeust cell
            CellOutput::new_builder()
                .lock(request_script)
                .type_(Some(personal_script).pack())
                .build_exact_capacity(Capacity::shannons(request_capacity))
                .unwrap(),
            // change cell
            CellOutput::new_builder()
                .lock(secp256k1_script.clone())
                .build_exact_capacity(Capacity::zero())
                .unwrap(),
        ];
        let outputs_data = vec![
            Bytes::from(previous_json_data.as_bytes().to_vec()),
            Bytes::new(),
        ];
        let outputs_capacity = helper::calc_outputs_capacity(&outputs, "1.0");

        // fill reqeust transaction inputs
        let search = SearchKey {
            script: secp256k1_script.into(),
            script_type: ScriptType::Lock,
            filter: None,
        };
        let (mut extra_inputs, inputs_capacity) =
            helper::fetch_live_cells(&self.rpc_client, search, inputs_capacity, outputs_capacity)
                .await?;
        inputs.append(&mut extra_inputs);

        // rebuild change output
        if inputs_capacity < outputs_capacity {
            return Err(
                BackendError::InsufficientCapacity(inputs_capacity, outputs_capacity).into(),
            );
        }
        let change = inputs_capacity - outputs_capacity;
        outputs[1] = outputs[1]
            .clone()
            .as_builder()
            .build_exact_capacity(Capacity::shannons(change))
            .unwrap();

        // build knside-out transaction
        let tx = TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs)
            .outputs_data(outputs_data.pack())
            .cell_deps(project_deps.project_cell_deps.clone())
            .build();

        // generate transaction digest
        let digest = helper::get_transaction_digest(&tx);
        self.cached_transactions.insert(digest.clone(), tx);

        Ok((digest, payment_ckb))
    }

    async fn send_transaction_to_ckb(
        &mut self,
        digest: &H256,
        signature: &[u8; 65],
    ) -> KoResult<Option<H256>> {
        let tx = self.cached_transactions.remove(digest);
        if let Some(tx) = tx {
            let tx = helper::complete_transaction_with_signature(tx, signature);
            let hash = self
                .rpc_client
                .send_transaction(&tx.data().into(), Some(OutputsValidator::Passthrough))
                .await
                .map_err(|err| {
                    BackendError::TransactionSendError(
                        err.to_string(),
                        to_string(&JsonTxView::from(tx)).unwrap(),
                    )
                })?;
            Ok(Some(hash))
        } else {
            Ok(None)
        }
    }

    async fn search_global_data(&self, project_deps: &ProjectDeps) -> KoResult<String> {
        // build global type_script search key
        let project_type_id =
            helper::recover_type_id_script(project_deps.project_type_args.as_bytes())
                .calc_script_hash()
                .unpack();
        let global_args = mol_flag_0(&project_type_id.0);
        let global_type_script =
            helper::build_knsideout_script(&project_deps.project_code_hash, &global_args);
        let search = SearchKey {
            script: global_type_script.into(),
            script_type: ScriptType::Type,
            filter: None,
        };

        // search global cell
        let result = self
            .rpc_client
            .fetch_live_cells(search, 1, None)
            .await
            .map_err(|err| BackendError::IndexerRpcError(err.to_string()))?;
        if result.objects.is_empty() {
            return Err(BackendError::MissProjectGlobalCell(
                project_deps.project_type_args.clone(),
            )
            .into());
        }

        // return global json_data
        let global_json_data = {
            let bytes = result.objects[0].output_data.as_bytes();
            String::from_utf8(bytes.to_vec()).map_err(|_| {
                BackendError::InvalidGlobalDataFormat(project_deps.project_type_args.clone())
            })?
        };
        Ok(global_json_data)
    }

    async fn search_personal_data(
        &self,
        address: String,
        project_deps: &ProjectDeps,
    ) -> KoResult<Vec<(String, OutPoint)>> {
        // build personal type_script search key
        let project_type_id =
            helper::recover_type_id_script(project_deps.project_type_args.as_bytes())
                .calc_script_hash()
                .unpack();
        let personal_args = mol_flag_1(&project_type_id.0);
        let personal_type_script =
            helper::build_knsideout_script(&project_deps.project_code_hash, &personal_args);
        let secp256k1_script: Script = Address::from_str(&address)
            .map_err(|_| BackendError::InvalidAddressFormat(address))?
            .payload()
            .into();
        let filter = SearchKeyFilter {
            script: Some(personal_type_script.into()),
            output_data_len_range: None,
            output_capacity_range: None,
            block_range: None,
        };
        let search = SearchKey {
            script: secp256k1_script.into(),
            script_type: ScriptType::Lock,
            filter: Some(filter),
        };

        // collect all personal cells
        let mut personal_json_data = vec![];
        let mut after = None;
        loop {
            let result = self
                .rpc_client
                .fetch_live_cells(search.clone(), 10, after)
                .await
                .map_err(|err| BackendError::IndexerRpcError(err.to_string()))?;
            result
                .objects
                .into_iter()
                .try_for_each::<_, KoResult<_>>(|cell| {
                    let json_data = {
                        let bytes = cell.output_data.as_bytes();
                        String::from_utf8(bytes.to_vec()).map_err(|_| {
                            BackendError::InvalidPersonalDataFormat(
                                project_deps.project_type_args.clone(),
                            )
                        })?
                    };
                    personal_json_data.push((json_data, cell.out_point.into()));
                    Ok(())
                })?;
            if result.last_cursor.is_empty() {
                break;
            }
            after = Some(result.last_cursor);
        }
        Ok(personal_json_data)
    }
}
