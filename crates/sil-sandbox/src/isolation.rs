//! Politique d'isolation d'un programme externe. Voir docs/technical/security/sandbox.md pour ce
//! que chaque plateforme applique réellement.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Ce que le programme a le droit de faire. Par défaut : rien hors de `writable`, rien hors de
/// `read_only`, pas de réseau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Isolation {
    /// Mémoire virtuelle maximale du processus, en octets (Linux).
    pub memory_bytes: u64,
    /// Taille maximale d'un fichier écrit, en octets (Linux).
    pub max_file_bytes: u64,
    /// Temps CPU maximal, en secondes (Linux).
    pub cpu_seconds: u64,
    /// Dossiers et fichiers lisibles hors du dossier de travail : le programme, ses bibliothèques.
    pub read_only: Vec<PathBuf>,
    /// Dossiers accessibles en lecture et en écriture : le dossier de travail, rien d'autre.
    pub writable: Vec<PathBuf>,
    /// Réseau autorisé ? Non par défaut.
    pub network: bool,
    /// `Strict` : refuser de lancer si une protection ne peut pas être appliquée.
    /// `BestEffort` : lancer avec les protections disponibles.
    pub level: Level,
    /// Lanceur qui applique la politique avant l'exécution (Linux). Absent : `Strict` échoue.
    pub launcher: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    Strict,
    BestEffort,
}

impl Isolation {
    /// Politique stricte pour `program`, qui n'écrit que dans `workdir`.
    pub fn for_program(program: &Path, workdir: &Path, launcher: Option<PathBuf>) -> Self {
        let mut read_only = vec![program.parent().unwrap_or(Path::new("/")).to_path_buf()];
        read_only.extend(system_read_only());
        Self {
            memory_bytes: 2 * 1024 * 1024 * 1024,
            max_file_bytes: 16 * 1024 * 1024,
            cpu_seconds: 600,
            read_only,
            writable: vec![workdir.to_path_buf()],
            network: false,
            level: Level::Strict,
            launcher,
        }
    }
}

/// Dossiers système nécessaires à un programme dynamique (bibliothèques, chargeur).
#[cfg(target_os = "linux")]
fn system_read_only() -> Vec<PathBuf> {
    [
        "/usr",
        "/lib",
        "/lib64",
        "/etc/ld.so.cache",
        "/dev/null",
        "/dev/urandom",
    ]
    .iter()
    .map(PathBuf::from)
    .collect()
}

#[cfg(not(target_os = "linux"))]
fn system_read_only() -> Vec<PathBuf> {
    Vec::new()
}

/// Lanceur par défaut : la variable `SIL_SANDBOX_LAUNCHER`, sinon `sil-sandbox-launch` à côté de
/// l'exécutable courant.
pub fn default_launcher() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("SIL_SANDBOX_LAUNCHER") {
        return Some(PathBuf::from(path));
    }
    let name = if cfg!(windows) {
        "sil-sandbox-launch.exe"
    } else {
        "sil-sandbox-launch"
    };
    let candidate = std::env::current_exe().ok()?.with_file_name(name);
    candidate.is_file().then_some(candidate)
}
