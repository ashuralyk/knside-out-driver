use std::collections::HashMap;
use std::str::FromStr;

use ko_protocol::ckb_jsonrpc_types::{OutputsValidator, TransactionView as JsonTxView};
use ko_protocol::ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey, SearchKeyFilter};
use ko_protocol::ckb_sdk::Address;
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::core::{Capacity, TransactionBuilder, TransactionView};
use ko_protocol::ckb_types::packed::{CellInput, CellOutput, OutPoint, Script, Transaction};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack, Unpack};
use ko_protocol::serde_json::to_string;
use ko_protocol::tokio::sync::mpsc::unbounded_channel;
use ko_protocol::traits::{Backend, CkbClient, ContextRpc};
use ko_protocol::types::backend::KoRequestInput;
use ko_protocol::{
    async_trait, hex, is_mol_request_identity, mol_identity, KoResult, ProjectDeps, H256,
};

#[cfg(test)]
mod tests;

mod error;
mod helper;
use error::BackendError;

pub struct BackendImpl<C: CkbClient, R: ContextRpc> {
    rpc_client: C,
    cached_transactions: HashMap<H256, TransactionView>,
    context_rpc: R,
}

impl<C: CkbClient, R: ContextRpc> BackendImpl<C, R> {
    pub fn new(rpc_client: &C, context_rpc: R) -> Self {
        BackendImpl {
            rpc_client: rpc_client.clone(),
            cached_transactions: HashMap::new(),
            context_rpc,
        }
    }

    pub fn peak_transaction(&self, digest: &H256) -> Option<TransactionView> {
        self.cached_transactions.get(digest).cloned()
    }
}

#[async_trait]
impl<C: CkbClient, R: ContextRpc> Backend for BackendImpl<C, R> {
    async fn create_project_deploy_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_deps: &ProjectDeps,
    ) -> KoResult<(H256, H256)> {
        // prepare scripts
        let ckb_address =
            Address::from_str(&address).map_err(|_| BackendError::InvalidAddressFormat(address))?;
        let secp256k1_script: Script = ckb_address.payload().into();
        let manager_secp256k1_script: Script = project_deps.project_manager.payload().into();

        // make global of output-data
        let owner = hex::encode(&secp256k1_script.calc_script_hash().raw_data());
        let manager = hex::encode(&manager_secp256k1_script.calc_script_hash().raw_data());
        let (global_data_json, owner_as_driver, contract_bytecode) =
            helper::get_global_json_data(&contract, &owner, &manager)?;

        // build mock knside-out transaction outputs and data
        let driver_secp256k1_script = if owner_as_driver {
            secp256k1_script.clone()
        } else {
            manager_secp256k1_script
        };
        let global_type_script = helper::build_knsideout_script(
            &project_deps.project_code_hash,
            mol_identity(0, &[0u8; 32]).as_slice(),
        );
        let mut outputs = vec![
            // project cell
            CellOutput::new_builder()
                .lock(secp256k1_script.clone())
                .type_(helper::build_type_id_script(None, 0))
                .build_exact_capacity(Capacity::bytes(contract_bytecode.len()).unwrap())
                .unwrap(),
            // global cell
            CellOutput::new_builder()
                .lock(driver_secp256k1_script)
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
            Bytes::from(contract_bytecode),
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
        if inputs_capacity < outputs_capacity {
            return Err(BackendError::InternalTransactionAssembleError.into());
        }

        // rebuild type_id with real input and change
        let project_type_script = helper::build_type_id_script(Some(&inputs[0]), 0)
            .to_opt()
            .unwrap();
        let project_type_args = {
            let args: Bytes = project_type_script.args().unpack();
            args.try_into().unwrap()
        };
        let project_type_id: H256 = {
            let hash = project_type_script.calc_script_hash();
            hash.unpack()
        };
        outputs[0] = outputs[0]
            .clone()
            .as_builder()
            .type_(Some(project_type_script).pack())
            .build();
        let global_type_script = helper::build_knsideout_script(
            &project_deps.project_code_hash,
            mol_identity(0, project_type_id.as_bytes32()).as_slice(),
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

    async fn create_project_upgrade_digest(
        &mut self,
        contract: Bytes,
        address: String,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<H256> {
        // search existed project deployment cell on CKB
        let project_type_script = helper::recover_type_id_script(project_type_args.as_bytes());
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
            return Err(BackendError::MissProjectDeploymentCell(project_type_args.clone()).into());
        }
        let deployment_cell = &result.objects[0];

        // build knside-out transaction outputs
        let secp256k1_script: Script = Address::from_str(&address)
            .map_err(|_| BackendError::InvalidAddressFormat(address))?
            .payload()
            .into();
        let previous_type_script = deployment_cell.output.type_.as_ref().unwrap();
        let contract_bytecode = helper::parse_contract_code(&contract)?;
        let mut outputs = vec![
            // new project deployment cell
            CellOutput::new_builder()
                .lock(secp256k1_script.clone())
                .type_(Some(previous_type_script.clone().into()).pack())
                .build_exact_capacity(Capacity::bytes(contract_bytecode.len()).unwrap())
                .unwrap(),
            // change cell
            CellOutput::new_builder()
                .lock(secp256k1_script.clone())
                .build_exact_capacity(Capacity::zero())
                .unwrap(),
        ];
        let outputs_data = vec![Bytes::from(contract_bytecode), Bytes::new()];
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
        function_call: String,
        input: KoRequestInput,
        component_outpoints: &[OutPoint],
        candidate_lockscripts: &[String],
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<(H256, u64)> {
        // build neccessary scripts
        let project_type_id: H256 = helper::recover_type_id_script(project_type_args.as_bytes())
            .calc_script_hash()
            .unpack();
        let candidates_script = candidate_lockscripts
            .iter()
            .map(|candidate| {
                Ok(Address::from_str(candidate)
                    .map_err(|_| BackendError::InvalidAddressFormat(candidate.clone()))?
                    .payload()
                    .into())
            })
            .collect::<Result<Vec<Script>, BackendError>>()?;
        let personal_args = mol_identity(1, project_type_id.as_bytes32());
        let personal_script =
            helper::build_knsideout_script(&project_deps.project_code_hash, &personal_args);

        // check input cells
        let mut inputs_capacity = 0u64;
        let mut inputs = vec![];
        let mut inputs_cell = vec![];
        match input {
            KoRequestInput::Address(address) => {
                let script: Script = Address::from_str(&address)
                    .map_err(|_| BackendError::InvalidAddressFormat(address))?
                    .payload()
                    .into();
                let (cell, ckb) = helper::fetch_cell_by_script(&self.rpc_client, &script).await?;
                inputs_cell.push((script, String::new()));
                inputs.push(cell);
                inputs_capacity = ckb;
            }
            KoRequestInput::Outpoints(outpoints) => {
                for out_point in outpoints {
                    let (cell, data, _) =
                        helper::fetch_outpoint_cell(&self.rpc_client, &out_point).await?;
                    let ckb: u64 = cell.capacity().unpack();
                    inputs_capacity += ckb;
                    inputs.push(
                        CellInput::new_builder()
                            .previous_output(out_point.clone())
                            .build(),
                    );
                    inputs_cell.push((cell.lock(), data));
                }
            }
        }
        if inputs.is_empty() {
            return Err(BackendError::MissInputCell.into());
        }

        // check component cells
        let mut components_data = vec![];
        let mut components = vec![];
        for out_point in component_outpoints {
            let (_, data, data_hash) =
                helper::fetch_outpoint_cell(&self.rpc_client, out_point).await?;
            if data.is_empty() {
                return Err(BackendError::InvalidComponentCell.into());
            }
            components_data.push(data);
            components.push((out_point, data_hash));
        }

        // request payment ckb of this call
        let payment_ckb = {
            let (sender, mut receiver) = unbounded_channel();
            let success = self
                .context_rpc
                .estimate_payment_ckb(
                    project_type_args,
                    &function_call,
                    &inputs_cell,
                    &candidates_script,
                    &components_data,
                    sender,
                )
                .await;
            if success {
                receiver.recv().await.unwrap()?
            } else {
                0u64
            }
        };

        // build request transaction outputs
        let request_args = mol_identity(2, project_type_id.as_bytes32());
        let request_script =
            helper::build_knsideout_script(&project_deps.project_code_hash, &request_args);
        let request_data = helper::make_request_data(
            &function_call,
            &inputs_cell,
            &components,
            &candidates_script,
        );
        let request_capacity = Capacity::bytes(request_data.len()).unwrap().as_u64() + payment_ckb;
        let mut outputs = vec![
            // request cell
            CellOutput::new_builder()
                .lock(request_script)
                .type_(Some(personal_script).pack())
                .build_exact_capacity(Capacity::shannons(request_capacity))
                .unwrap(),
            // change cell
            CellOutput::new_builder()
                .lock(inputs_cell[0].0.clone())
                .build_exact_capacity(Capacity::zero())
                .unwrap(),
        ];
        let outputs_data = vec![Bytes::from(request_data), Bytes::new()];
        let outputs_capacity = helper::calc_outputs_capacity(&outputs, "1.0");

        // fill request transaction inputs
        let search = SearchKey {
            script: inputs_cell[0].0.clone().into(),
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

    async fn check_project_request_committed(
        &mut self,
        transaction_hash: &H256,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<Option<H256>> {
        let out_point = OutPoint::new_builder()
            .tx_hash(transaction_hash.pack())
            .index(0u32.pack())
            .build();
        let cell = self
            .rpc_client
            .get_live_cell(&out_point.into(), false)
            .await?;
        let mut find = false;
        if let Some(cell) = cell.cell {
            let lock = CellOutput::from(cell.output).lock();
            if lock.code_hash() == project_deps.project_code_hash.pack()
                && lock.args().get(0) == Some(2u8.into())
            {
                find = true;
            }
        } else {
            let tx = self.rpc_client.get_transaction(transaction_hash).await?;
            if let Some(tx) = tx {
                if let Some(tx) = tx.transaction {
                    let tx = Transaction::from(tx.inner).into_view();
                    if let Some(cell) = tx.output(0) {
                        if cell.lock().code_hash() == project_deps.project_code_hash.pack()
                            && is_mol_request_identity(&cell.lock().args().raw_data())
                        {
                            find = true;
                        }
                    }
                }
            }
        }
        if find {
            let (sender, mut receiver) = unbounded_channel();
            let success = self
                .context_rpc
                .listen_request_committed(project_type_args, transaction_hash, sender)
                .await;
            if success {
                let committed_hash = receiver.recv().await.unwrap()?;
                return Ok(Some(committed_hash));
            } else {
                return Ok(None);
            }
        }
        Err(BackendError::InvalidRequestHash(transaction_hash.clone()).into())
    }

    async fn drive_project_on_management(
        &mut self,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<()> {
        let global_type_script =
            helper::build_global_type_script(&project_deps.project_code_hash, project_type_args);
        let global_lock_script: Script = project_deps.project_manager.payload().into();
        let filter = SearchKeyFilter {
            script: Some(global_type_script.into()),
            output_data_len_range: None,
            output_capacity_range: None,
            block_range: None,
        };
        let search_key = SearchKey {
            script: global_lock_script.into(),
            script_type: ScriptType::Lock,
            filter: Some(filter),
        };
        let result = self
            .rpc_client
            .fetch_live_cells(search_key, 1, None)
            .await
            .map_err(|err| BackendError::IndexerRpcError(err.to_string()))?;
        if result.objects.is_empty() {
            return Err(BackendError::MissManagedGlobalCell(project_type_args.clone()).into());
        }
        if !self
            .context_rpc
            .start_project_driver(project_type_args)
            .await
        {
            return Err(BackendError::AlreadyManagedProject(project_type_args.clone()).into());
        }
        Ok(())
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

    async fn search_global_data(
        &self,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<String> {
        // build global type_script search key
        let project_type_id: H256 = helper::recover_type_id_script(project_type_args.as_bytes())
            .calc_script_hash()
            .unpack();
        let global_args = mol_identity(0, project_type_id.as_bytes32());
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
            return Err(BackendError::MissProjectGlobalCell(project_type_args.clone()).into());
        }

        // return global json_data
        let global_json_data = {
            let bytes = result.objects[0].output_data.as_bytes();
            String::from_utf8(bytes.to_vec())
                .map_err(|_| BackendError::InvalidGlobalDataFormat(project_type_args.clone()))?
        };
        Ok(global_json_data)
    }

    async fn search_personal_data(
        &self,
        address: String,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> KoResult<Vec<(String, OutPoint)>> {
        // build personal type_script search key
        let project_type_id: H256 = helper::recover_type_id_script(project_type_args.as_bytes())
            .calc_script_hash()
            .unpack();
        let personal_args = mol_identity(1, project_type_id.as_bytes32());
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
                            BackendError::InvalidPersonalDataFormat(project_type_args.clone())
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
