use crate::{ConfigError, NodeConfig, env, validate::validate};
use std::path::Path;

/// Charge la config : fichier optionnel, puis surcharges via `get` (ex. `std::env::var`).
pub fn load(
    path: Option<&Path>,
    get: &dyn Fn(&str) -> Option<String>,
) -> Result<NodeConfig, ConfigError> {
    let text = match path {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|_| ConfigError::Unreadable(path.to_path_buf()))?,
        None => String::new(),
    };
    load_from_str(&text, get)
}

/// Comme [`load`], depuis un texte TOML (tests, config embarquée).
pub fn load_from_str(
    text: &str,
    get: &dyn Fn(&str) -> Option<String>,
) -> Result<NodeConfig, ConfigError> {
    let mut config: NodeConfig =
        toml::from_str(text).map_err(|error| ConfigError::Parse(error.message().to_string()))?;
    env::apply(&mut config, get)?;
    validate(&config)?;
    Ok(config)
}
