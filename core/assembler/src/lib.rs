use ko_protocol::ckb_sdk::rpc::ckb_indexer::IndexerRpcClient;
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack, Unpack};
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::core::{Capacity, TransactionView, DepType};
use ko_protocol::ckb_types::packed::{CellInput, CellOutput, OutPoint, Script, CellDep, WitnessArgs};
use ko_protocol::traits::Assembler;
use ko_protocol::types::assembler::{KoAssembleReceipt, KoProject, KoRequest};
use ko_protocol::KoResult;

mod error;
mod helper;

use error::AssemblerError;

pub struct AssemblerImpl {
    rpc_client: IndexerRpcClient,
    project_id: [u8; 32],
    project_code_hash: [u8; 32]
}

impl AssemblerImpl {
    pub fn new(indexer_url: &str, project_id: &[u8; 32], code_hash: &[u8; 32]) -> AssemblerImpl {
        AssemblerImpl {
            rpc_client: IndexerRpcClient::new(indexer_url),
            project_id: project_id.clone(),
            project_code_hash: code_hash.clone()
        }
    }
}

impl Assembler for AssemblerImpl {
    fn prepare_ko_transaction_project_celldep(
        &mut self,
        project_deployment_args: &[u8; 32]
    ) -> KoResult<KoProject> {
        let project_cell = 
            helper::search_project_cell(&mut self.rpc_client, project_deployment_args)?;
        let project_celldep = CellDep::new_builder()
            .out_point(project_cell.out_point)
            .dep_type(DepType::Code.into())
            .build();
        let project_lua_code = helper::extract_project_lua_code(&project_cell.output_data)?;
        Ok(KoProject::new(project_celldep, project_lua_code))
    }

    fn generate_ko_transaction_with_inputs_and_celldeps(
        &mut self,
        txs: &Vec<TransactionView>,
        cell_deps: &Vec<CellDep>
    ) -> KoResult<(TransactionView, KoAssembleReceipt)> {
        // find project global cell
        let global_cell =
            helper::search_global_cell(&mut self.rpc_client, &self.project_code_hash, &self.project_id)?;
        let mut ko_tx = TransactionView::new_advanced_builder()
            .input(
                CellInput::new_builder()
                    .previous_output(global_cell.out_point)
                    .build(),
            )
            .cell_deps(cell_deps.clone())
            .build();
        // fill transaction inputs and collect KnsideOut requests
        let mut requests = vec![];
        for tx in txs {
            for i in 0..tx.outputs().len() {
                if let Some(output) = tx.outputs().get(i) {
                    if helper::check_valid_request(&output, &self.project_code_hash, &self.project_id) {
                        let cell_data = tx.outputs_data().get(i).unwrap().raw_data();
                        let flag_2 =
                            ko_protocol::mol_flag_2_raw(&output.lock().args().raw_data().to_vec())
                                .unwrap();
                        let lock_script = Script::from_slice(&flag_2.caller_lockscript().raw_data())
                            .map_err(|_| AssemblerError::UnsupportedCallerScriptFormat)?;
                        let capacity = output.capacity().unpack();
                        requests.push(
                            KoRequest::new(
                                cell_data,
                                flag_2.function_call().raw_data(), 
                                lock_script,
                                capacity
                            )
                        );
                        let out_point = OutPoint::new_builder()
                            .tx_hash(tx.hash())
                            .index((i as u32).pack())
                            .build();
                        ko_tx = tx
                            .as_advanced_builder()
                            .input(CellInput::new_builder().previous_output(out_point).build())
                            .build();
                    }
                }
            }
        }
        let receipt = KoAssembleReceipt::new(
            requests,
            global_cell.output_data,
            global_cell.output.calc_lock_hash().unpack()
        );
        Ok((ko_tx, receipt))
    }

    fn fill_ko_transaction_with_outputs(
        &self,
        mut tx: TransactionView,
        outputs_data: &Vec<Bytes>,
        inputs_capacity: u64,
        lock_scripts: &Vec<Script>,
    ) -> KoResult<TransactionView> {
        // have an initial check of input params
        if lock_scripts.len() != outputs_data.len() || lock_scripts.len() != tx.inputs().len() {
            return Err(
                AssemblerError::ScriptsAndOutputsDataMismatch(
                    lock_scripts.len(),
                    outputs_data.len(),
                ).into()
            );
        }
        // collect output cells and their total capacity
        let mut outputs = vec![];
        let mut outputs_capacity = 0u64;
        lock_scripts.iter().enumerate().for_each(|(i, lock)| {
            let type_ = if i == 0 {
                helper::make_global_script(&self.project_code_hash, &self.project_id)
            } else {
                helper::make_personal_script(&self.project_code_hash, &self.project_id)
            };
            if outputs_data[i].is_empty() {
                outputs.push(
                    CellOutput::new_builder()
                        .lock(lock.clone())
                        .build_exact_capacity(Capacity::zero())
                        .unwrap(),
                );
            } else {
                outputs.push(
                    CellOutput::new_builder()
                        .lock(lock.clone())
                        .type_(Some(type_).pack())
                        .build_exact_capacity(Capacity::bytes(outputs_data[i].len()).unwrap())
                        .unwrap(),
                );
            }
            let capacity: u64 = outputs.last().unwrap().capacity().unpack();
            outputs_capacity += capacity;
        });
        // firstly check inputs/outputs capacity
        if inputs_capacity <= outputs_capacity {
            return Err(AssemblerError::TransactionCapacityError(
                inputs_capacity,
                outputs_capacity,
            ).into());
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
            ).into());
        }
        Ok(tx)
    }

    fn complete_ko_transaction_with_signature(
        &self,
        tx: TransactionView,
        signature: Bytes
    ) -> TransactionView {
        let witness = WitnessArgs::new_builder()
            .lock(Some(signature).pack())
            .build()
            .as_bytes();
        tx
            .as_advanced_builder()
            .witness(witness.pack())
            .build()
    }
}
