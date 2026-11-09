use crate::{ConfigError, NodeConfig};

const LEVELS: [&str; 5] = ["error", "warn", "info", "debug", "trace"];

fn invalid(field: &'static str, reason: &str) -> ConfigError {
    ConfigError::Invalid {
        field,
        reason: reason.to_string(),
    }
}

/// Refuse toute configuration dangereuse ou incohérente avant de démarrer.
pub(crate) fn validate(config: &NodeConfig) -> Result<(), ConfigError> {
    if config.data_dir.as_os_str().is_empty() {
        return Err(invalid("data_dir", "vide"));
    }
    if config.control.port == 0 {
        return Err(invalid("control.port", "doit être non nul"));
    }
    if !config.control.bind.is_loopback() && config.control.token_file.is_none() {
        return Err(invalid(
            "control.token_file",
            "obligatoire quand control.bind n'est pas en boucle locale",
        ));
    }
    if config.network.listen.is_empty() {
        return Err(invalid("network.listen", "au moins une adresse"));
    }
    if let Some(address) = config
        .network
        .bootstrap
        .iter()
        .find(|address| !address.contains("/p2p/"))
    {
        return Err(ConfigError::Invalid {
            field: "network.bootstrap",
            reason: format!("{address} n'épingle pas le pair (/p2p/<id> requis)"),
        });
    }
    let limits = &config.limits;
    if limits.max_message_bytes == 0 || limits.max_peers == 0 {
        return Err(invalid("limits", "valeurs nulles interdites"));
    }
    if limits.max_ledger_bytes == 0 || limits.max_content_bytes == 0 {
        return Err(invalid("limits", "valeurs nulles interdites"));
    }
    for requester in &config.trust.requesters {
        let hex = requester.len() == 64
            && requester
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !hex {
            return Err(invalid(
                "trust.requesters",
                "clé publique de 64 caractères hex minuscules attendue",
            ));
        }
    }
    if let Some(chain) = &config.chain {
        if !(chain.rpc_url.starts_with("http://") || chain.rpc_url.starts_with("https://")) {
            return Err(invalid("chain.rpc_url", "http:// ou https:// attendu"));
        }
        if chain.program_id.trim().is_empty() {
            return Err(invalid("chain.program_id", "obligatoire"));
        }
        if let Some(payer) = &chain.payer_keypair {
            if payer.as_os_str().is_empty() {
                return Err(invalid("chain.payer_keypair", "chemin vide"));
            }
        }
        if let Some(relay) = &chain.relay_url {
            if !(relay.starts_with("http://") || relay.starts_with("https://")) {
                return Err(invalid("chain.relay_url", "http:// ou https:// attendu"));
            }
        }
        if chain.payer_keypair.is_none() && chain.relay_url.is_none() {
            return Err(invalid(
                "chain",
                "payer_keypair (ce nœud règle) ou relay_url (règlement par un relais) requis",
            ));
        }
    }
    if config.testnet.free_work && config.chain.is_some() {
        return Err(invalid(
            "testnet.free_work",
            "interdit avec [chain] : le travail doit être payé dès qu'une chaîne existe",
        ));
    }
    if let Some(render) = &config.render {
        if render.engine.as_os_str().is_empty() {
            return Err(invalid("render.engine", "obligatoire"));
        }
        if render.wall_ms == 0 || render.wall_ms > 3_600_000 {
            return Err(invalid("render.wall_ms", "entre 1 et 3600000"));
        }
    }
    if !LEVELS.contains(&config.log.level.as_str()) {
        return Err(invalid("log.level", "error|warn|info|debug|trace"));
    }
    Ok(())
}
