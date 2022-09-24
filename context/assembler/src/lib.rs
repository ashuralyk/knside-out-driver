use ckb_hash::{Blake2bBuilder, CKB_HASH_PERSONALIZATION};

use helper::{clone_with_new_capacity, fill_transaction_capacity_diff, get_extractable_capacity};
use ko_protocol::ckb_sdk::constants::TYPE_ID_CODE_HASH;
use ko_protocol::ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey};
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::core::{Capacity, DepType, ScriptHashType, TransactionView};
use ko_protocol::ckb_types::packed::{CellDep, CellInput, CellOutput, Script, WitnessArgs};
use ko_protocol::ckb_types::prelude::{Builder, Entity, Pack, Unpack};
use ko_protocol::traits::{Assembler, CkbClient};
use ko_protocol::types::assembler::{KoAssembleReceipt, KoCellOutput, KoProject, KoRequest};
use ko_protocol::types::context::KoContextGlobalCell;
use ko_protocol::{async_trait, KoResult, ProjectDeps, H256};

mod error;
mod helper;

use error::AssemblerError;

pub struct AssemblerImpl<C: CkbClient> {
    rpc_client: C,
    project_id: H256,
    project_id_args: H256,
    project_code_hash: H256,
    project_cell_deps: Vec<CellDep>,
}

impl<C: CkbClient> AssemblerImpl<C> {
    pub fn new(
        rpc_client: &C,
        project_type_args: &H256,
        project_deps: &ProjectDeps,
    ) -> AssemblerImpl<C> {
        let project_id = Script::new_builder()
            .code_hash(TYPE_ID_CODE_HASH.pack())
            .hash_type(ScriptHashType::Type.into())
            .args(project_type_args.as_bytes().pack())
            .build()
            .calc_script_hash()
            .unpack();
        AssemblerImpl {
            project_id,
            project_id_args: project_type_args.clone(),
            rpc_client: rpc_client.clone(),
            project_code_hash: project_deps.project_code_hash.clone(),
            project_cell_deps: project_deps.project_cell_deps.clone(),
        }
    }

    pub fn get_project_id(&self) -> &H256 {
        &self.project_id
    }

    pub fn get_project_args(&self) -> &H256 {
        &self.project_id_args
    }

    pub async fn get_project_global_cell(&self) -> KoResult<KoContextGlobalCell> {
        let global_cell =
            helper::search_global_cell(&self.rpc_client, &self.project_code_hash, &self.project_id)
                .await?;
        Ok(global_cell.into())
    }
}

#[async_trait]
impl<C: CkbClient> Assembler for AssemblerImpl<C> {
    async fn prepare_transaction_project_celldep(&self) -> KoResult<KoProject> {
        let project_cell =
            helper::search_project_cell(&self.rpc_client, &self.project_id_args).await?;
        let project_celldep = CellDep::new_builder()
            .out_point(project_cell.out_point)
            .dep_type(DepType::Code.into())
            .build();
        Ok(KoProject::new(
            project_celldep,
            project_cell.output_data,
            project_cell.output.lock(),
        ))
    }

    async fn generate_transaction_with_inputs_and_celldeps(
        &self,
        cell_number: u8,
        extra_cell_dep: &CellDep,
    ) -> KoResult<(TransactionView, KoAssembleReceipt)> {
        // find project global cell
        let global_cell =
            helper::search_global_cell(&self.rpc_client, &self.project_code_hash, &self.project_id)
                .await?;
        let cell_deps = {
            let mut cell_deps = self.project_cell_deps.clone();
            cell_deps.push(extra_cell_dep.clone());
            cell_deps
        };
        let mut tx = TransactionView::new_advanced_builder()
            .input(
                CellInput::new_builder()
                    .previous_output(global_cell.out_point.clone())
                    .build(),
            )
            .cell_deps(cell_deps)
            .build();

        // fill transaction inputs and collect KnsideOut requests
        let mut requests = vec![];
        let search_key = SearchKey {
            script: helper::make_personal_script(&self.project_code_hash, &self.project_id).into(),
            script_type: ScriptType::Type,
            filter: None,
        };
        let mut after = None;
        let mut blake2b = Blake2bBuilder::new(16)
            .personal(CKB_HASH_PERSONALIZATION)
            .build();
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
                    if !helper::check_valid_request(&output, &self.project_code_hash) {
                        // println!("[WARN] find invalid reqeust format");
                        return Ok(());
                    }
                    let flag_2 = ko_protocol::mol_flag_2_raw(&output.lock().args().raw_data())
                        .expect("flag_2 molecule");
                    let caller_lockscript =
                        Script::from_slice(&flag_2.caller_lockscript().raw_data())
                            .map_err(|_| AssemblerError::UnsupportedCallerScriptFormat)?;
                    let recipient_lockscript = {
                        let script = flag_2.recipient_lockscript().to_opt();
                        if let Some(inner) = script {
                            let script = Script::from_slice(&inner.raw_data())
                                .map_err(|_| AssemblerError::UnsupportedRecipientScriptFormat)?;
                            Some(script)
                        } else {
                            None
                        }
                    };
                    let payment_ckb = {
                        let capacity: u64 = output.capacity().unpack();
                        let exact_capacity = output
                            .occupied_capacity(Capacity::bytes(cell.output_data.len()).unwrap())
                            .unwrap()
                            .as_u64();
                        capacity - exact_capacity
                    };
                    requests.push(KoRequest::new(
                        cell.output_data.into_bytes(),
                        flag_2.function_call().raw_data(),
                        caller_lockscript,
                        recipient_lockscript,
                        payment_ckb,
                        output.capacity().unpack(),
                    ));
                    let input = CellInput::new_builder()
                        .previous_output(cell.out_point.into())
                        .build();
                    blake2b.update(input.as_slice());
                    tx = tx.as_advanced_builder().input(input).build();
                    Ok(())
                })?;
            if result.last_cursor.is_empty() {
                break;
            }
            after = Some(result.last_cursor);
        }

        // make random seed
        let mut random_bytes = [0u8; 16];
        blake2b.finalize(&mut random_bytes);
        let receipt = KoAssembleReceipt::new(requests, global_cell.into(), random_bytes);
        Ok((tx, receipt))
    }

    async fn fill_transaction_with_outputs(
        &self,
        mut tx: TransactionView,
        cell_outputs: &[KoCellOutput],
        inputs_capacity: u64,
        fee: u64,
    ) -> KoResult<TransactionView> {
        // collect output cells and their total capacity
        let mut outputs = vec![];
        let mut outputs_capacity = fee;
        let mut outputs_data = vec![];
        cell_outputs.iter().enumerate().for_each(|(i, output)| {
            let mut cell_output = CellOutput::new_builder()
                .lock(output.lock_script.clone())
                .build_exact_capacity(Capacity::zero())
                .unwrap();
            let mut cell_output_data = Bytes::new();

            // handle cell which will contain personal json data
            if let Some(data) = &output.data {
                let type_ = if i == 0 {
                    helper::make_global_script(&self.project_code_hash, &self.project_id)
                } else {
                    helper::make_personal_script(&self.project_code_hash, &self.project_id)
                };
                cell_output = cell_output
                    .as_builder()
                    .type_(Some(type_).pack())
                    .build_exact_capacity(Capacity::bytes(data.len()).unwrap())
                    .unwrap();
                cell_output_data = data.clone();
            }

            // handle cell in which the capacity needs to use the one from request
            if output.capacity > cell_output.capacity().unpack() {
                cell_output = cell_output
                    .as_builder()
                    .capacity(Capacity::shannons(output.capacity).pack())
                    .build();
            }
            let capacity: u64 = cell_output.capacity().unpack();
            outputs_capacity += capacity;
            outputs.push(cell_output);
            outputs_data.push(cell_output_data);
        });

        // check inputs/outputs capacity
        let capacity: u64 = outputs[0].capacity().unpack();
        if inputs_capacity >= outputs_capacity {
            outputs[0] =
                clone_with_new_capacity(&outputs[0], inputs_capacity - outputs_capacity + capacity);
        } else {
            let diff = outputs_capacity - inputs_capacity;
            let change_room = get_extractable_capacity(&outputs[0], outputs_data[0].len());
            if change_room >= diff {
                outputs[0] = clone_with_new_capacity(&outputs[0], capacity - diff);
            } else {
                outputs[0] = clone_with_new_capacity(&outputs[0], capacity - change_room);
                fill_transaction_capacity_diff(
                    &self.rpc_client,
                    &outputs[0].lock(),
                    diff - change_room,
                    &mut tx,
                    &mut outputs,
                    &mut outputs_data,
                )
                .await?;
            }
        }

        // complete partial tx
        tx = tx
            .as_advanced_builder()
            .outputs(outputs)
            .outputs_data(outputs_data.pack())
            .build();

        Ok(tx)
    }

    fn complete_transaction_with_signature(
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
