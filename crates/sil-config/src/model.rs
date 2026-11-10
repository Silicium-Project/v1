use serde::Deserialize;
use std::{net::IpAddr, path::PathBuf};

/// Configuration complète d'un nœud.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct NodeConfig {
    /// Dossier des données (clés, journal, contenu). Réservé à l'utilisateur du nœud.
    pub data_dir: PathBuf,
    pub network: NetworkConfig,
    pub control: ControlConfig,
    pub limits: LimitsConfig,
    pub log: LogConfig,
    pub trust: TrustConfig,
    /// Chaîne Solana pour le règlement des jobs. Absente : le nœud ne règle rien.
    pub chain: Option<ChainConfig>,
    /// Moteur de rendu. Absent : le nœud ne fait pas de `rendu/raytracer`.
    pub render: Option<RenderConfig>,
    /// Mode de test. Par défaut désactivé.
    pub testnet: TestnetConfig,
    /// Télémétrie de la machine vers l'API. Absente : rien n'est envoyé.
    pub telemetry: Option<TelemetryConfig>,
}

/// Envoi périodique de l'état de la machine (CPU, mémoire, rôles, tâches) à l'API. Le message est
/// signé par la clé du nœud ; l'API ne l'accepte que pour un appareil lié à un compte.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryConfig {
    /// Adresse de base de l'API (https://…). Le chemin /v1/devices/telemetry y est ajouté.
    pub api_url: String,
    /// Période d'envoi, en secondes.
    #[serde(default = "default_telemetry_interval_s")]
    pub interval_s: u64,
}

fn default_telemetry_interval_s() -> u64 {
    60
}

/// Modes de test. Refusés si une chaîne est configurée (voir `validate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct TestnetConfig {
    /// Le worker calcule sans vérifier d'escrow : le travail n'est pas payé. Réseau de test seulement.
    pub free_work: bool,
}

/// Moteur de rendu : un programme installé par l'opérateur, lancé dans `sil-sandbox`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderConfig {
    /// Chemin du programme (protocole : voir `sil-wl-render::ExternalRenderer`).
    pub engine: PathBuf,
    /// Durée maximale d'un bloc rendu, en millisecondes.
    #[serde(default = "default_render_wall_ms")]
    pub wall_ms: u64,
    /// Isolation noyau du moteur. `strict` (défaut) refuse de lancer si elle ne peut pas être
    /// complète ; `best-effort` lance avec ce qui existe ; `off` ne l'applique pas (développement).
    #[serde(default)]
    pub isolation: IsolationMode,
}

/// Niveau d'isolation demandé pour le moteur de rendu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum IsolationMode {
    #[default]
    Strict,
    BestEffort,
    Off,
}

fn default_render_wall_ms() -> u64 {
    600_000
}

/// Chaîne Solana : où régler les jobs et avec quelle clé payer les frais.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainConfig {
    /// URL RPC du validateur ou du réseau (http:// ou https://).
    pub rpc_url: String,
    /// Identifiant (base58) du programme d'escrow.
    pub program_id: String,
    /// Fichier JSON de la clé qui paie les frais de transaction (comme `solana-keygen`). Absent :
    /// ce nœud ne règle pas lui-même, il envoie ses règlements à `relay_url`.
    pub payer_keypair: Option<PathBuf>,
    /// Relais de règlement (`https://…`) : `POST /v1/settlements` règle avec les frais du relais.
    pub relay_url: Option<String>,
}

/// Confiance accordée par ce nœud.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct TrustConfig {
    /// Requesters (clé publique, 64 caractères hex minuscules) dont ce nœud croit les journaux
    /// pour calculer la réputation des workers. Vide = aucune réputation.
    pub requesters: Vec<String>,
}

/// Réseau P2P. Les adresses sont des multiaddr, validées par `sil-net`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct NetworkConfig {
    pub listen: Vec<String>,
    /// Pairs d'amorçage ; chaque adresse doit épingler le pair (`/p2p/<id>`). Ils servent aussi
    /// de relais : le nœud y réserve un circuit, ce qui le rend joignable même derrière un NAT.
    pub bootstrap: Vec<String>,
    /// Ce nœud sert de relais public. À activer seulement s'il est joignable directement.
    pub relay_server: bool,
    pub enable_mdns: bool,
}

/// API HTTP de contrôle.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ControlConfig {
    pub bind: IpAddr,
    pub port: u16,
    /// Fichier contenant le jeton. Obligatoire hors boucle locale.
    pub token_file: Option<PathBuf>,
}

/// Bornes de ressources.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LimitsConfig {
    pub max_message_bytes: usize,
    pub max_peers: usize,
    pub max_ledger_bytes: u64,
    pub max_content_bytes: u64,
}

/// Journalisation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LogConfig {
    /// `error`, `warn`, `info`, `debug` ou `trace`.
    pub level: String,
    pub json: bool,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("data"),
            network: NetworkConfig::default(),
            control: ControlConfig::default(),
            limits: LimitsConfig::default(),
            log: LogConfig::default(),
            trust: TrustConfig::default(),
            chain: None,
            render: None,
            testnet: TestnetConfig::default(),
            telemetry: None,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen: vec!["/ip4/0.0.0.0/udp/4001/quic-v1".into()],
            bootstrap: Vec::new(),
            relay_server: false,
            enable_mdns: false,
        }
    }
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            bind: IpAddr::from([127, 0, 0, 1]),
            port: 8750,
            token_file: None,
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_message_bytes: 64 * 1024,
            max_peers: 128,
            max_ledger_bytes: 2 * 1024 * 1024 * 1024,
            max_content_bytes: 20 * 1024 * 1024 * 1024,
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
            json: false,
        }
    }
}
