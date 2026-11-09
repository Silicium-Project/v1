//! Configuration du nœud écrite avant chaque démarrage (chemins en barres obliques, TOML lisible
//! par le nœud). Les adresses du réseau et de la chaîne sont fixées ici.

use crate::node_process::CONTROL_PORT;
use std::path::Path;

const LISTEN: &str = "/ip4/0.0.0.0/tcp/4001";

/// First node of the network (the VPS). Its peer id is pinned in the address.
const BOOTSTRAP: &str =
    "/ip4/213.32.68.67/tcp/4001/p2p/12D3KooWHXaQz2ou13jcnXq33yZbHKSmabBJJYxuLhx4UhG2k8ob";
/// Solana devnet: the escrow program and its RPC. No real value moves on it.
const CHAIN_RPC: &str = "https://api.devnet.solana.com";
const PROGRAM_ID: &str = "7WzVtrXToN9wvK1TGp2RMH6R6zzodaQU6TmtpNAgNfKR";
/// Relais de règlement : ce PC n'a pas de clé de paiement, le relais paie les frais de ses règlements.
const RELAY: &str = "https://siliciumlabs.org";

/// The node configuration written before each start. Paths use forward slashes (valid on
/// Windows and parsed the same way by the node's TOML reader). With an engine, the render
/// section is set: best-effort isolation, since strict needs Linux.
pub fn node_config(data_dir: &Path, engine: Option<&Path>) -> String {
    let dir = data_dir.to_string_lossy().replace('\\', "/");
    let render = engine.map_or_else(String::new, |path| {
        let engine = path.to_string_lossy().replace('\\', "/");
        format!("\n[render]\nengine = \"{engine}\"\nisolation = \"best-effort\"\n")
    });
    format!(
        "data_dir = \"{dir}\"\n\n[network]\nlisten = [\"{LISTEN}\"]\nbootstrap = [\"{BOOTSTRAP}\"]\n\n[control]\nbind = \"127.0.0.1\"\nport = {CONTROL_PORT}\ntoken_file = \"{dir}/control.token\"\n\n[chain]\nrpc_url = \"{CHAIN_RPC}\"\nprogram_id = \"{PROGRAM_ID}\"\nrelay_url = \"{RELAY}\"\n{render}"
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "tests fail loudly on purpose")]
mod tests {
    use super::*;

    #[test]
    fn config_points_the_node_at_its_folder_and_the_loopback_api() {
        let text = node_config(Path::new("C:\\Users\\x\\node"), None);
        assert!(text.contains("data_dir = \"C:/Users/x/node\""));
        assert!(text.contains("bind = \"127.0.0.1\""));
        assert!(text.contains("port = 8750"));
        assert!(text.contains("token_file = \"C:/Users/x/node/control.token\""));
        assert!(!text.contains("[render]"));
    }

    #[test]
    fn the_node_joins_the_network_and_settles_through_the_relay() {
        let text = node_config(Path::new("C:\\Users\\x\\node"), None);
        assert!(text.contains(&format!("bootstrap = [\"{BOOTSTRAP}\"]")));
        assert!(text.contains("[chain]\nrpc_url = \"https://api.devnet.solana.com\""));
        assert!(text.contains("relay_url = \"https://siliciumlabs.org\""));
        assert!(!text.contains("payer_keypair"));
    }

    #[test]
    fn an_engine_next_to_the_app_enables_rendering_best_effort() {
        let text = node_config(
            Path::new("C:\\Users\\x\\node"),
            Some(Path::new(
                "C:\\Program Files\\Silicium\\sil-raytracer-block.exe",
            )),
        );
        assert!(text.contains(
            "[render]\nengine = \"C:/Program Files/Silicium/sil-raytracer-block.exe\"\nisolation = \"best-effort\"\n"
        ));
    }
}
