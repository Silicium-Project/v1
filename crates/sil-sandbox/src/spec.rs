use crate::Isolation;
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Clone, Copy)]
pub struct SandboxLimits {
    /// Durée maximale d'exécution.
    pub wall: Duration,
    /// Octets maximum lus sur stdout, puis sur stderr.
    pub max_stdout: usize,
    pub max_stderr: usize,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            wall: Duration::from_secs(60),
            max_stdout: 64 * 1024 * 1024,
            max_stderr: 64 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SandboxSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    /// Répertoire de travail (doit exister).
    pub workdir: PathBuf,
    /// Seules variables d'environnement transmises (l'environnement du nœud n'est pas hérité).
    pub env: Vec<(String, String)>,
    /// Octets écrits sur l'entrée standard.
    pub stdin: Vec<u8>,
    pub limits: SandboxLimits,
    /// Isolation noyau. Absente : seulement l'environnement, le délai et la sortie sont bornés.
    pub isolation: Option<Isolation>,
}

#[derive(Debug)]
pub struct RunOutcome {
    /// Code de sortie (`None` si terminé par un signal).
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub elapsed: Duration,
}

impl RunOutcome {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}
