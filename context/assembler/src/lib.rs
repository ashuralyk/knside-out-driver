use ckb_hash::{Blake2bBuilder, CKB_HASH_PERSONALIZATION};

use helper::{clone_with_new_capacity, fill_transaction_capacity_diff, get_extractable_capacity};
use ko_protocol::ckb_sdk::constants::TYPE_ID_CODE_HASH;
use ko_protocol::ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey};
use ko_protocol::ckb_types::bytes::Bytes;
use ko_protocol::ckb_types::core::{Capacity, DepType, ScriptHashType, TransactionView};
use ko_protocol::ckb_types::packed::{CellDep, CellInput, Script, WitnessArgs};
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
    project_manager: Script,
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
            project_manager: project_deps.project_manager.payload().into(),
        }
    }

    pub fn get_project_id(&self) -> &H256 {
        &self.project_id
    }

    pub fn get_project_args(&self) -> &H256 {
        &self.project_id_args
    }

    pub async fn get_project_global_cell(&self) -> KoResult<KoContextGlobalCell> {
        let global_cell = helper::search_global_cell(
            &self.rpc_client,
            &self.project_code_hash,
            &self.project_id,
            Some(&self.project_manager),
        )
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
        let global_cell = helper::search_global_cell(
            &self.rpc_client,
            &self.project_code_hash,
            &self.project_id,
            Some(&self.project_manager),
        )
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
            for cell in result.objects.into_iter() {
                let output = cell.output.into();
                let output_data = cell.output_data.as_bytes();
                if !helper::check_valid_request(&output, output_data, &self.project_code_hash) {
                    // println!("[WARN] find invalid reqeust format");
                    continue;
                }
                let request = ko_protocol::parse_mol_request(output_data);
                let inputs = helper::extract_inputs_from_request(&request)?;
                let candidates = helper::extract_candidates_from_request(&request)?;
                let components =
                    helper::extract_components_from_request(&self.rpc_client, &request).await?;
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
                    inputs,
                    candidates,
                    components,
                    payment_ckb,
                    output.capacity().unpack(),
                ));
                let input = CellInput::new_builder()
                    .previous_output(cell.out_point.into())
                    .build();
                blake2b.update(input.as_slice());
                tx = tx.as_advanced_builder().input(input).build();
            }
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
            let (mut cells, mut data, capacity) =
                helper::process_raw_outputs(i, output, &self.project_code_hash, &self.project_id);
            outputs_capacity += capacity;
            outputs.append(&mut cells);
            outputs_data.append(&mut data);
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
