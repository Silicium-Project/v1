//! Surcharges par variables d'environnement. Le lecteur est injecté : pas d'accès global.

use crate::{ConfigError, NodeConfig};

fn parse<T: std::str::FromStr>(name: &'static str, raw: &str) -> Result<T, ConfigError>
where
    T::Err: std::fmt::Display,
{
    raw.trim()
        .parse()
        .map_err(|error: T::Err| ConfigError::BadEnv {
            name,
            reason: error.to_string(),
        })
}

/// Applique les variables `SIL_*` présentes.
pub(crate) fn apply(
    config: &mut NodeConfig,
    get: &dyn Fn(&str) -> Option<String>,
) -> Result<(), ConfigError> {
    if let Some(raw) = get("SIL_DATA_DIR") {
        config.data_dir = raw.trim().into();
    }
    if let Some(raw) = get("SIL_CONTROL_BIND") {
        config.control.bind = parse("SIL_CONTROL_BIND", &raw)?;
    }
    if let Some(raw) = get("SIL_CONTROL_PORT") {
        config.control.port = parse("SIL_CONTROL_PORT", &raw)?;
    }
    if let Some(raw) = get("SIL_CONTROL_TOKEN_FILE") {
        config.control.token_file = Some(raw.trim().into());
    }
    if let Some(raw) = get("SIL_LOG_LEVEL") {
        config.log.level = raw.trim().to_ascii_lowercase();
    }
    if let Some(raw) = get("SIL_LOG_JSON") {
        config.log.json = parse("SIL_LOG_JSON", &raw)?;
    }
    Ok(())
}
