use ko_protocol::ckb_sdk::constants::TYPE_ID_CODE_HASH;
use ko_protocol::ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey};
use ko_protocol::ckb_types::core::{Capacity, DepType, ScriptHashType, TransactionView};
use ko_protocol::ckb_types::packed::{CellDep, CellInput, CellOutput, Script, WitnessArgs};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack, Unpack};
use ko_protocol::ckb_types::{bytes::Bytes, H256};
use ko_protocol::traits::{Assembler, CkbClient};
use ko_protocol::types::assembler::{KoAssembleReceipt, KoCellOutput, KoProject, KoRequest};
use ko_protocol::{async_trait, KoResult};

mod error;
mod helper;

use error::AssemblerError;

pub struct AssemblerImpl<C: CkbClient> {
    rpc_client: C,
    project_id: H256,
    project_code_hash: H256,
}

impl<C: CkbClient> AssemblerImpl<C> {
    pub fn new(rpc_client: &C, project_args: &H256, code_hash: &H256) -> AssemblerImpl<C> {
        let project_id = Script::new_builder()
            .code_hash(TYPE_ID_CODE_HASH.pack())
            .hash_type(ScriptHashType::Type.into())
            .args(project_args.as_bytes().pack())
            .build()
            .calc_script_hash()
            .unpack();
        AssemblerImpl {
            project_id,
            rpc_client: rpc_client.clone(),
            project_code_hash: code_hash.clone(),
        }
    }
}

#[async_trait]
impl<C: CkbClient> Assembler for AssemblerImpl<C> {
    async fn prepare_ko_transaction_project_celldep(
        &self,
        project_deployment_args: &H256,
    ) -> KoResult<KoProject> {
        let project_cell =
            helper::search_project_cell(&self.rpc_client, project_deployment_args).await?;
        let project_celldep = CellDep::new_builder()
            .out_point(project_cell.out_point)
            .dep_type(DepType::Code.into())
            .build();
        let project_lua_code = helper::extract_project_lua_code(&project_cell.output_data)?;
        Ok(KoProject::new(project_celldep, project_lua_code))
    }

    async fn generate_ko_transaction_with_inputs_and_celldeps(
        &self,
        cell_number: u8,
        cell_deps: &[CellDep],
    ) -> KoResult<(TransactionView, KoAssembleReceipt)> {
        // find project global cell
        let global_cell =
            helper::search_global_cell(&self.rpc_client, &self.project_code_hash, &self.project_id)
                .await?;
        let mut ko_tx = TransactionView::new_advanced_builder()
            .input(
                CellInput::new_builder()
                    .previous_output(global_cell.out_point)
                    .build(),
            )
            .cell_deps(cell_deps.to_vec())
            .build();
        // fill transaction inputs and collect KnsideOut requests
        let mut requests = vec![];
        let search_key = SearchKey {
            script: helper::make_personal_script(&self.project_code_hash, &self.project_id).into(),
            script_type: ScriptType::Type,
            filter: None,
        };
        let mut total_inputs_capacity: u64 = global_cell.output.capacity().unpack();
        let mut after = None;
        while requests.len() < cell_number as usize {
            let result = self
                .rpc_client
                .fetch_live_cells(search_key.clone(), 30, after)
                .await
                .map_err(|_| AssemblerError::MissProjectRequestCell)?;
            result
                .objects
                .into_iter()
                .try_for_each::<_, KoResult<_>>(|cell| {
                    let output = cell.output.into();
                    if helper::check_valid_request(
                        &output,
                        &self.project_code_hash,
                        &self.project_id,
                    ) {
                        let flag_2 =
                            ko_protocol::mol_flag_2_raw(&output.lock().args().raw_data().to_vec())
                                .unwrap();
                        let lock_script =
                            Script::from_slice(&flag_2.caller_lockscript().raw_data())
                                .map_err(|_| AssemblerError::UnsupportedCallerScriptFormat)?;
                        let payment = {
                            let capacity: u64 = output.capacity().unpack();
                            total_inputs_capacity += capacity;
                            let exact_capacity =
                                Capacity::bytes(output.as_bytes().len() + cell.output_data.len())
                                    .unwrap()
                                    .as_u64();
                            capacity - exact_capacity
                        };
                        requests.push(KoRequest::new(
                            cell.output_data.into_bytes(),
                            flag_2.function_call().raw_data(),
                            lock_script,
                            payment,
                        ));
                        ko_tx = ko_tx
                            .as_advanced_builder()
                            .input(
                                CellInput::new_builder()
                                    .previous_output(cell.out_point.into())
                                    .build(),
                            )
                            .build();
                    }
                    Ok(())
                })?;
            if result.last_cursor.is_empty() {
                break;
            }
            after = Some(result.last_cursor);
        }
        let receipt = KoAssembleReceipt::new(
            requests,
            global_cell.output_data,
            global_cell.output.lock(),
            total_inputs_capacity,
        );
        Ok((ko_tx, receipt))
    }

    async fn fill_ko_transaction_with_outputs(
        &self,
        mut tx: TransactionView,
        cell_outputs: &[KoCellOutput],
        inputs_capacity: u64,
    ) -> KoResult<TransactionView> {
        // collect output cells and their total capacity
        let mut outputs = vec![];
        let mut outputs_capacity = 0u64;
        let mut outputs_data = vec![];
        cell_outputs.iter().enumerate().for_each(|(i, output)| {
            let type_ = if i == 0 {
                helper::make_global_script(&self.project_code_hash, &self.project_id)
            } else {
                helper::make_personal_script(&self.project_code_hash, &self.project_id)
            };
            if output.data.is_empty() {
                outputs.push(
                    CellOutput::new_builder()
                        .lock(output.lock_script.clone())
                        .build_exact_capacity(Capacity::shannons(output.payment))
                        .unwrap(),
                );
            } else {
                let capacity = {
                    let capacity = Capacity::bytes(output.data.len()).unwrap();
                    capacity
                        .safe_add(Capacity::shannons(output.payment))
                        .unwrap()
                };
                outputs.push(
                    CellOutput::new_builder()
                        .lock(output.lock_script.clone())
                        .type_(Some(type_).pack())
                        .build_exact_capacity(capacity)
                        .unwrap(),
                );
            }
            let capacity: u64 = outputs.last().unwrap().capacity().unpack();
            outputs_capacity += capacity;
            outputs_data.push(output.data.clone());
        });
        // firstly check inputs/outputs capacity
        if inputs_capacity <= outputs_capacity {
            return Err(AssemblerError::TransactionCapacityError(
                inputs_capacity,
                outputs_capacity,
            )
            .into());
        }
        // complete transaction outputs
        let fee = Capacity::bytes(1).unwrap().as_u64();
        if !outputs.is_empty() {
            let global_capacity: u64 = {
                let capacity: u64 = outputs[0].capacity().unpack();
                inputs_capacity - outputs_capacity - fee + capacity
            };
            outputs[0] = outputs[0]
                .clone()
                .as_builder()
                .capacity(global_capacity.pack())
                .build();
        }
        tx = tx
            .as_advanced_builder()
            .outputs(outputs)
            .outputs_data(outputs_data.pack())
            .build();
        // secondly check inputs/outputs capacity as a barrier
        if inputs_capacity <= tx.outputs_capacity().unwrap().as_u64() {
            return Err(AssemblerError::TransactionCapacityError(
                inputs_capacity,
                tx.outputs_capacity().unwrap().as_u64(),
            )
            .into());
        }
        Ok(tx)
    }

    fn complete_ko_transaction_with_signature(
        &self,
        tx: TransactionView,
        signature: Bytes,
    ) -> TransactionView {
        let witness = WitnessArgs::new_builder()
            .lock(Some(signature).pack())
            .build()
            .as_bytes();
        tx.as_advanced_builder().witness(witness.pack()).build()
    }
}
