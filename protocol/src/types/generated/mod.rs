#![allow(dead_code)]

#[allow(clippy::all)]
pub mod generated;

use molecule::prelude::{Builder, Byte, Entity, Reader};

fn mol_hash(v: &[u8; 32]) -> generated::Hash {
    let mut mol_bytes: [Byte; 32] = [Byte::default(); 32];
    for i in 0..32 {
        mol_bytes[i] = Byte::from(v[i]);
    }
    generated::Hash::new_builder().set(mol_bytes).build()
}

fn mol_string(v: &[u8]) -> generated::String {
    let bytes = v
        .to_vec()
        .iter()
        .map(|byte| Byte::new(*byte))
        .collect::<Vec<Byte>>();
    generated::String::new_builder().set(bytes).build()
}

fn mol_string_opt(v: Option<&[u8]>) -> generated::StringOpt {
    if let Some(v) = v {
        let string = mol_string(v);
        generated::StringOpt::new_builder()
            .set(Some(string))
            .build()
    } else {
        generated::StringOpt::new_builder().set(None).build()
    }
}

fn mol_string_vec(v: &[&[u8]]) -> generated::StringVec {
    let strings = v.iter().map(|bytes| mol_string(bytes)).collect::<Vec<_>>();
    generated::StringVec::new_builder().set(strings).build()
}

fn mol_cell(lockscript: &[u8], data: Option<&[u8]>) -> generated::Cell {
    generated::Cell::new_builder()
        .owner_lockscript(mol_string(lockscript))
        .data(mol_string_opt(data))
        .build()
}

fn mol_cell_vec(v: &[(&[u8], Option<&[u8]>)]) -> generated::CellVec {
    let cells = v
        .iter()
        .map(|(lock, data)| mol_cell(lock, *data))
        .collect::<Vec<_>>();
    generated::CellVec::new_builder().set(cells).build()
}

fn mol_celldep(tx_hash: &[u8; 32], index: u8, data_hash: &[u8; 32]) -> generated::Celldep {
    generated::Celldep::new_builder()
        .tx_hash(mol_hash(tx_hash))
        .index(index.into())
        .data_hash(mol_hash(data_hash))
        .build()
}

fn mol_celldep_vec(celldeps: &[(&[u8; 32], u8, &[u8; 32])]) -> generated::CelldepVec {
    let celldeps = celldeps
        .iter()
        .map(|(tx_hash, index, data_hash)| mol_celldep(tx_hash, *index, data_hash))
        .collect::<Vec<_>>();
    generated::CelldepVec::new_builder().set(celldeps).build()
}

pub fn mol_identity(flag: u8, hash: &[u8; 32]) -> Vec<u8> {
    generated::Identity::new_builder()
        .flag(flag.into())
        .project_id(mol_hash(hash))
        .build()
        .as_bytes()
        .to_vec()
}

pub fn mol_request(
    method: &str,
    cells: &[(&[u8], Option<&[u8]>)],
    cell_deps: &[(&[u8; 32], u8, &[u8; 32])],
    floatings: &[&[u8]],
) -> Vec<u8> {
    generated::Request::new_builder()
        .function_call(mol_string(method.as_bytes()))
        .cells(mol_cell_vec(cells))
        .function_celldeps(mol_celldep_vec(cell_deps))
        .floating_lockscripts(mol_string_vec(floatings))
        .build()
        .as_bytes()
        .to_vec()
}

pub fn is_mol_request(bytes: &[u8]) -> bool {
    generated::RequestReader::verify(bytes, false).is_ok()
}

pub fn is_mol_request_identity(bytes: &[u8]) -> bool {
    if generated::IdentityReader::verify(bytes, false).is_ok() {
        let identity = generated::Identity::new_unchecked(bytes.to_vec().into());
        if identity.flag() == 2u8.into() {
            return true;
        }
    }
    false
}

pub fn parse_mol_request(bytes: &[u8]) -> generated::Request {
    generated::Request::new_unchecked(bytes.to_vec().into())
}
