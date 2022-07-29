#![allow(dead_code)]

#[allow(clippy::all)]
pub mod generated;

use molecule::bytes::Bytes;
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

fn mol_project_info(
    name: &str,
    author: &str,
    website: &str,
    description: &str,
) -> generated::ProjectInfo {
    generated::ProjectInfo::new_builder()
        .name(mol_string(name.as_bytes()))
        .author(mol_string(author.as_bytes()))
        .website(mol_string(website.as_bytes()))
        .description(mol_string(description.as_bytes()))
        .build()
}

pub fn mol_deployment(lua_code: &[u8]) -> generated::Deployment {
    let project_info = mol_project_info("", "", "", "");
    generated::Deployment::new_builder()
        .code(mol_string(lua_code))
        .project(project_info)
        .build()
}

pub fn mol_flag_0(hash: &[u8; 32]) -> Vec<u8> {
    let mut flag_0_bytes = generated::Flag0::new_builder()
        .project_id(mol_hash(hash))
        .build()
        .as_bytes()
        .to_vec();
    flag_0_bytes.insert(0, 0u8);
    flag_0_bytes
}

pub fn mol_flag_1(hash: &[u8; 32]) -> Vec<u8> {
    let mut flag_1_bytes = generated::Flag1::new_builder()
        .project_id(mol_hash(hash))
        .build()
        .as_bytes()
        .to_vec();
    flag_1_bytes.insert(0, 1u8);
    flag_1_bytes
}

pub fn mol_flag_2(hash: &[u8; 32], method: &str, lockscript: &[u8]) -> Vec<u8> {
    let mut flag_2_bytes = generated::Flag2::new_builder()
        .project_id(mol_hash(hash))
        .function_call(mol_string(method.as_bytes()))
        .caller_lockscript(mol_string(lockscript))
        .build()
        .as_bytes()
        .to_vec();
    flag_2_bytes.insert(0, 2u8);
    flag_2_bytes
}

pub fn mol_deployment_raw(bytes: &[u8]) -> Option<generated::Deployment> {
    if generated::DeploymentReader::verify(bytes, false).is_ok() {
        Some(generated::Deployment::new_unchecked(Bytes::from(
            bytes.to_vec(),
        )))
    } else {
        None
    }
}

pub fn mol_flag_2_raw(bytes: &[u8]) -> Option<generated::Flag2> {
    let payload = &bytes[1..];
    if generated::Flag2Reader::verify(payload, false).is_ok() {
        Some(generated::Flag2::new_unchecked(Bytes::from(bytes.to_vec())))
    } else {
        None
    }
}

pub fn is_mol_flag_0(bytes: &[u8], hash: Option<&[u8; 32]>) -> bool {
    if !bytes.is_empty()
        && bytes[0] == 0u8
        && generated::Flag0Reader::verify(&bytes[1..], false).is_ok()
    {
        if let Some(hash) = hash {
            let flag_0 = generated::Flag0::new_unchecked(Bytes::from(bytes[1..].to_vec()));
            if flag_0.project_id().as_slice() == hash.as_slice() {
                return true;
            }
        } else {
            return true;
        }
    }
    false
}

pub fn is_mol_flag_1(bytes: &[u8], hash: Option<&[u8; 32]>) -> bool {
    if !bytes.is_empty()
        && bytes[0] == 1u8
        && generated::Flag1Reader::verify(&bytes[1..], false).is_ok()
    {
        if let Some(hash) = hash {
            let flag_1 = generated::Flag1::new_unchecked(Bytes::from(bytes[1..].to_vec()));
            if flag_1.project_id().as_slice() == hash.as_slice() {
                return true;
            }
        } else {
            return true;
        }
    }
    false
}

pub fn is_mol_flag_2(bytes: &[u8]) -> bool {
    if !bytes.is_empty() && bytes[0] == 2u8 {
        generated::Flag2Reader::verify(&bytes[1..], false).is_ok()
    } else {
        false
    }
}
