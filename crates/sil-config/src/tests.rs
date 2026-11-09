use crate::{ConfigError, load_from_str};
use std::collections::HashMap;

fn no_env(_: &str) -> Option<String> {
    None
}

fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    move |name| map.get(name).cloned()
}

#[test]
fn defaults_are_valid() {
    let config = load_from_str("", &no_env).unwrap();
    assert_eq!(config.control.port, 8750);
    assert!(config.control.bind.is_loopback());
}

#[test]
fn environment_overrides_the_file() {
    let get = env_of(&[("SIL_CONTROL_PORT", "9000"), ("SIL_LOG_LEVEL", "DEBUG")]);
    let config = load_from_str("[control]\nport = 8000\n", &get).unwrap();
    assert_eq!(config.control.port, 9000);
    assert_eq!(config.log.level, "debug");
}

#[test]
fn unknown_keys_are_refused() {
    assert!(matches!(
        load_from_str("surprise = 1\n", &no_env),
        Err(ConfigError::Parse(_))
    ));
}

#[test]
fn bad_environment_value_is_named() {
    let get = env_of(&[("SIL_CONTROL_PORT", "abc")]);
    assert!(matches!(
        load_from_str("", &get),
        Err(ConfigError::BadEnv {
            name: "SIL_CONTROL_PORT",
            ..
        })
    ));
}

#[test]
fn exposed_api_requires_a_token_file() {
    let exposed = "[control]\nbind = \"0.0.0.0\"\n";
    assert!(matches!(
        load_from_str(exposed, &no_env),
        Err(ConfigError::Invalid {
            field: "control.token_file",
            ..
        })
    ));
    let protected = "[control]\nbind = \"0.0.0.0\"\ntoken_file = \"token\"\n";
    assert!(load_from_str(protected, &no_env).is_ok());
}

#[test]
fn bootstrap_must_pin_the_peer() {
    let unpinned = "[network]\nbootstrap = [\"/ip4/1.2.3.4/udp/4001/quic-v1\"]\n";
    assert!(load_from_str(unpinned, &no_env).is_err());
    let pinned = "[network]\nbootstrap = [\"/ip4/1.2.3.4/udp/4001/quic-v1/p2p/12D3KooWabc\"]\n";
    assert!(load_from_str(pinned, &no_env).is_ok());
}

#[test]
fn zero_limits_and_bad_levels_are_refused() {
    assert!(load_from_str("[limits]\nmax_peers = 0\n", &no_env).is_err());
    assert!(load_from_str("[log]\nlevel = \"loud\"\n", &no_env).is_err());
}

#[test]
fn trusted_requesters_must_be_lowercase_hex_keys() {
    let good = format!(
        "[trust]
requesters = [\"{}\"]
",
        "ab".repeat(32)
    );
    assert_eq!(
        load_from_str(&good, &no_env)
            .unwrap()
            .trust
            .requesters
            .len(),
        1
    );
    for bad in ["abc", &"AB".repeat(32), &"zz".repeat(32)] {
        let text = format!(
            "[trust]
requesters = [\"{bad}\"]
"
        );
        assert!(load_from_str(&text, &no_env).is_err(), "{bad}");
    }
    assert!(
        load_from_str("", &no_env)
            .unwrap()
            .trust
            .requesters
            .is_empty()
    );
}

#[test]
fn a_chain_section_needs_an_http_rpc_a_program_and_a_payer() {
    let valid = "[chain]\nrpc_url = \"http://127.0.0.1:8899\"\nprogram_id = \"4nLifxCqFRnNiwnktuiLiH9gr2XvX1rpcQvNXgZgz4jB\"\npayer_keypair = \"payer.json\"\n";
    let config = load_from_str(valid, &no_env).unwrap();
    assert_eq!(config.chain.unwrap().rpc_url, "http://127.0.0.1:8899");
    assert!(
        load_from_str("", &no_env).unwrap().chain.is_none(),
        "no chain section, no settlement"
    );

    let ftp = valid.replace("http://127.0.0.1:8899", "ftp://x");
    assert!(matches!(
        load_from_str(&ftp, &no_env),
        Err(ConfigError::Invalid {
            field: "chain.rpc_url",
            ..
        })
    ));
    let no_program = valid.replace(
        "program_id = \"4nLifxCqFRnNiwnktuiLiH9gr2XvX1rpcQvNXgZgz4jB\"",
        "program_id = \" \"",
    );
    assert!(matches!(
        load_from_str(&no_program, &no_env),
        Err(ConfigError::Invalid {
            field: "chain.program_id",
            ..
        })
    ));
}

#[test]
fn free_work_is_off_by_default_and_refused_with_a_chain() {
    assert!(
        !load_from_str("", &no_env).unwrap().testnet.free_work,
        "désactivé par défaut"
    );
    let free = "[testnet]\nfree_work = true\n";
    assert!(load_from_str(free, &no_env).unwrap().testnet.free_work);
    let chain = "[chain]\nrpc_url = \"http://127.0.0.1:8899\"\nprogram_id = \"4nLifxCqFRnNiwnktuiLiH9gr2XvX1rpcQvNXgZgz4jB\"\npayer_keypair = \"payer.json\"\n";
    assert!(matches!(
        load_from_str(&format!("{free}{chain}"), &no_env),
        Err(ConfigError::Invalid {
            field: "testnet.free_work",
            ..
        })
    ));
}

#[test]
fn the_solo_mode_is_gone_and_its_key_is_refused() {
    assert!(load_from_str("[testnet]\nsolo_verify = true\n", &no_env).is_err(), "plus de mode solo (ADR 0007)");
}

#[test]
fn a_render_section_needs_an_engine_and_a_sane_deadline() {
    let valid = "[render]\nengine = \"sil-render-reference\"\n";
    let config = load_from_str(valid, &no_env).unwrap();
    let render = config.render.expect("section lue");
    assert_eq!(render.wall_ms, 600_000, "délai par défaut");
    assert!(
        load_from_str("", &no_env).unwrap().render.is_none(),
        "pas de section, pas de rendu"
    );
    let no_engine = valid.replace("sil-render-reference", "");
    assert!(matches!(
        load_from_str(&no_engine, &no_env),
        Err(ConfigError::Invalid {
            field: "render.engine",
            ..
        })
    ));
    let too_long = format!("{valid}wall_ms = 4000000\n");
    assert!(matches!(
        load_from_str(&too_long, &no_env),
        Err(ConfigError::Invalid {
            field: "render.wall_ms",
            ..
        })
    ));
}

#[test]
fn a_node_settles_with_its_own_payer_or_through_a_relay() {
    let relay_only = "[chain]\nrpc_url = \"http://127.0.0.1:8899\"\nprogram_id = \"4nLifxCqFRnNiwnktuiLiH9gr2XvX1rpcQvNXgZgz4jB\"\nrelay_url = \"https://siliciumlabs.org\"\n";
    let config = load_from_str(relay_only, &no_env).unwrap();
    let chain = config.chain.unwrap();
    assert!(chain.payer_keypair.is_none());
    assert_eq!(chain.relay_url.as_deref(), Some("https://siliciumlabs.org"));

    let neither = relay_only.replace("relay_url = \"https://siliciumlabs.org\"\n", "");
    assert!(matches!(
        load_from_str(&neither, &no_env),
        Err(ConfigError::Invalid { field: "chain", .. })
    ));

    let bad_relay = relay_only.replace("https://siliciumlabs.org", "ftp://x");
    assert!(matches!(
        load_from_str(&bad_relay, &no_env),
        Err(ConfigError::Invalid {
            field: "chain.relay_url",
            ..
        })
    ));
}
