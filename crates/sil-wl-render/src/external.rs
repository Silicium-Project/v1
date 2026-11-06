//! Moteur de rendu externe : un programme installé par l'opérateur, lancé par `sil-sandbox`.
//!
//! Protocole : arguments `<largeur> <hauteur> <y0> <y1>` ; la scène sur l'entrée standard ; sur
//! la sortie standard, exactement `largeur × (y1 - y0) × 3` octets RGB24. Tout autre
//! comportement (délai dépassé, sortie trop grande, code de sortie non nul, taille fausse) est une
//! erreur : la bande n'est pas rendue, donc pas réglée.
//!
//! Ce que `sil-sandbox` ne garantit pas (réseau, mémoire, fichiers) : voir
//! docs/technical/security/sandbox.md. Le moteur tourne avec les droits du nœud.

use crate::BlockRenderer;
use sil_sandbox::{Isolation, SandboxError, SandboxLimits, SandboxSpec, run};
use sil_workload_api::WorkloadError;
use std::{path::PathBuf, time::Duration};

/// Plafond de la sortie d'erreur du moteur : il ne doit pas remplir la mémoire du nœud.
const MAX_STDERR: usize = 64 * 1024;

pub struct ExternalRenderer {
    program: PathBuf,
    workdir: PathBuf,
    wall: Duration,
    extra_args: Vec<String>,
    isolation: Option<Isolation>,
}

impl ExternalRenderer {
    /// `program` : le moteur. `workdir` : son dossier de travail (doit exister). `wall` : durée
    /// maximale d'un bloc.
    pub fn new(program: PathBuf, workdir: PathBuf, wall: Duration) -> Self {
        Self {
            program,
            workdir,
            wall,
            extra_args: Vec::new(),
            isolation: None,
        }
    }

    /// Arguments ajoutés après les quatre du protocole. Réservé aux tests du moteur de référence
    /// (modes de panne) : la configuration du nœud ne les expose pas.
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    /// Lance le moteur avec cette isolation noyau (voir `sil_sandbox::Isolation`).
    pub fn with_isolation(mut self, isolation: Isolation) -> Self {
        self.isolation = Some(isolation);
        self
    }
}

impl BlockRenderer for ExternalRenderer {
    fn render(
        &self,
        scene: &[u8],
        width: u32,
        height: u32,
        y0: u32,
        y1: u32,
    ) -> Result<Vec<u8>, WorkloadError> {
        let expected = usize::try_from(u64::from(width) * u64::from(y1 - y0) * 3)
            .map_err(|_| WorkloadError::OverLimit("taille du bloc"))?;
        let mut args: Vec<String> = [width, height, y0, y1].iter().map(u32::to_string).collect();
        args.extend(self.extra_args.iter().cloned());
        let spec = SandboxSpec {
            program: self.program.clone(),
            args,
            workdir: self.workdir.clone(),
            // Environnement vide : le moteur ne voit ni jetons ni variables du nœud.
            env: Vec::new(),
            stdin: scene.to_vec(),
            limits: SandboxLimits {
                wall: self.wall,
                // La sortie est exactement la bande : tout octet de plus tue le moteur.
                max_stdout: expected,
                max_stderr: MAX_STDERR,
            },
            isolation: self.isolation.clone(),
        };
        let outcome = run(&spec).map_err(map_error)?;
        if !outcome.success() {
            let reason = String::from_utf8_lossy(&outcome.stderr);
            return Err(WorkloadError::Execution(format!(
                "moteur : code {:?} {}",
                outcome.code,
                reason.chars().take(200).collect::<String>()
            )));
        }
        if outcome.stdout.len() != expected {
            return Err(WorkloadError::Execution(format!(
                "moteur : {} octets rendus, {expected} attendus",
                outcome.stdout.len()
            )));
        }
        Ok(outcome.stdout)
    }
}

fn map_error(error: SandboxError) -> WorkloadError {
    match error {
        SandboxError::Timeout => WorkloadError::Execution("moteur : délai dépassé".into()),
        SandboxError::OutputTooLarge { .. } => {
            WorkloadError::OverLimit("sortie du moteur trop grande")
        }
        SandboxError::IsolationUnavailable(reason) => {
            WorkloadError::Execution(format!("moteur : isolation impossible, {reason}"))
        }
        other => WorkloadError::Execution(other.to_string()),
    }
}
