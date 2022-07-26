use ko_protocol::ckb_types::H256;
use ko_protocol::KoResult;
use serde::Deserialize;
use toml;

mod error;
use error::ConfigError;

#[derive(Deserialize)]
pub struct KoCellDep {
    pub transaction_hash: H256,
    pub cell_index: u32,
}

#[derive(Deserialize)]
pub struct KoConfig {
    pub project_type_args: H256,
    pub project_type_id: H256,
    pub project_owner_privkey: H256,
    pub project_code_hash: H256,
    pub ckb_url: String,
    pub ckb_indexer_url: String,
    pub project_cell_deps: Vec<KoCellDep>,
}

pub fn load_file(path: &str) -> KoResult<KoConfig> {
    let file = std::fs::read_to_string(path)
        .map_err(|err| ConfigError::ErrorLoadingConfig(path.into(), err.to_string()))?;
    let config: KoConfig = toml::from_str(file.as_str())
        .map_err(|err| ConfigError::ErrorLoadingConfig(path.into(), err.to_string()))?;
    Ok(config)
}
