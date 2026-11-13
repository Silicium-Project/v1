//! Arguments de l'outil. Tout a une valeur par défaut, sauf rien : l'outil tourne sans argument
//! contre le devnet et le programme déployé.

use std::path::PathBuf;

/// Programme d'escrow déployé sur devnet (voir chain/README.md).
pub const DEVNET_PROGRAM: &str = "5nvVoget71j65HtMZmfVJjnwSePDtrG5hzAPpjMbrMkx";
pub const DEVNET_RPC: &str = "https://api.devnet.solana.com";

pub struct Config {
    /// API publique (tunnel SSH vers le VPS, ou adresse directe).
    pub api: String,
    pub rpc: String,
    pub program: String,
    /// Graine maîtresse du requester de test (32 octets, mode 0600, hors dépôt).
    pub seed: PathBuf,
    /// Taille de l'entrée, en octets.
    pub bytes: usize,
    pub fragments: u32,
    /// Délai d'attente de chaque étape, en secondes.
    pub timeout_s: u64,
    /// Mode lecture seule : affiche l'escrow on-chain d'un job existant (identifiant hex).
    pub read_job: Option<String>,
    /// Workload du job : `calcul/sha256` (par défaut) ou `rendu/raytracer` (avec `scene`).
    pub workload: String,
    /// Scène `.cfg` à rendre (workload `rendu/raytracer`).
    pub scene: Option<PathBuf>,
    /// Dimensions de l'image, celles de la scène (workload `rendu/raytracer`).
    pub width: u32,
    pub height: u32,
    /// Hors chain : pas d'escrow ni de règlement, le réseau doit tourner en `free_work`.
    pub off_chain: bool,
}

impl Config {
    /// Lit `--option valeur` ; refuse une option inconnue ou une valeur hors bornes.
    pub fn from_args(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut cfg = Self {
            api: "http://127.0.0.1:8080".into(),
            rpc: DEVNET_RPC.into(),
            program: DEVNET_PROGRAM.into(),
            seed: home().join("sil-devnet").join("requester.seed"),
            bytes: 2048,
            fragments: 2,
            timeout_s: 900,
            read_job: None,
            workload: "calcul/sha256".into(),
            scene: None,
            width: 0,
            height: 0,
            off_chain: false,
        };
        while let Some(flag) = args.next() {
            if flag == "--off-chain" {
                cfg.off_chain = true;
                continue;
            }
            let value = args.next().ok_or(format!("{flag} attend une valeur"))?;
            match flag.as_str() {
                "--api" => cfg.api = value,
                "--rpc" => cfg.rpc = value,
                "--program" => cfg.program = value,
                "--seed" => cfg.seed = value.into(),
                "--bytes" => cfg.bytes = number(&flag, &value)?,
                "--fragments" => cfg.fragments = number(&flag, &value)?,
                "--timeout" => cfg.timeout_s = number(&flag, &value)?,
                "--read" => cfg.read_job = Some(value),
                "--workload" => cfg.workload = value,
                "--scene" => cfg.scene = Some(value.into()),
                "--width" => cfg.width = number(&flag, &value)?,
                "--height" => cfg.height = number(&flag, &value)?,
                other => return Err(format!("option inconnue : {other}")),
            }
        }
        if cfg.workload == "rendu/raytracer"
            && (cfg.scene.is_none() || cfg.width == 0 || cfg.height == 0)
        {
            return Err("rendu/raytracer : --scene, --width et --height sont requis".into());
        }
        if !(1..=16).contains(&cfg.fragments) {
            return Err("--fragments : entre 1 et 16".into());
        }
        if cfg.bytes == 0 || cfg.bytes > 1 << 20 {
            return Err("--bytes : entre 1 et 1048576".into());
        }
        Ok(cfg)
    }
}

fn number<T: std::str::FromStr>(flag: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} : nombre attendu"))
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}
