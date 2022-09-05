use ko_protocol::types::config::{KoConfig, KoConfigTypeArgs, KoTypeArgsItem};
use ko_protocol::{KoResult, H256};

mod error;
use error::ConfigError;

pub fn load_file(path: &str) -> KoResult<KoConfig> {
    let file = std::fs::read_to_string(path)
        .map_err(|err| ConfigError::LoadingConfig(path.into(), err.to_string()))?;
    let config: KoConfig = toml::from_str(file.as_str())
        .map_err(|err| ConfigError::LoadingConfig(path.into(), err.to_string()))?;
    Ok(config)
}

pub fn load_type_args_file(path: &str) -> KoResult<KoConfigTypeArgs> {
    let file = std::fs::read_to_string(path)
        .map_err(|err| ConfigError::LoadingConfigTypeArgs(path.into(), err.to_string()))?;
    let config: KoConfigTypeArgs = toml::from_str(file.as_str())
        .map_err(|err| ConfigError::LoadingConfigTypeArgs(path.into(), err.to_string()))?;
    Ok(config)
}

pub fn save_type_args_file(type_args_items: Vec<(H256, bool)>, path: &str) -> KoResult<()> {
    let values = type_args_items
        .into_iter()
        .map(|(hash, enable)| KoTypeArgsItem::new(hash, enable))
        .collect();
    let config = KoConfigTypeArgs::new(values);
    let file = toml::to_string_pretty(&config)
        .map_err(|err| ConfigError::SavingConfigTypeArgs(err.to_string()))?;
    std::fs::write(path, file).map_err(|err| ConfigError::SavingConfigTypeArgs(err.to_string()))?;
    Ok(())
}
