use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("répertoire de travail introuvable : {0}")]
    WorkdirMissing(PathBuf),
    #[error("lancement impossible : {0}")]
    Spawn(std::io::Error),
    #[error("délai dépassé, processus tué")]
    Timeout,
    #[error("sortie {stream} au-delà du plafond, processus tué")]
    OutputTooLarge { stream: &'static str },
    #[error("sortie du processus illisible : {0}")]
    Io(String),
    #[error("chemin refusé : {0}")]
    BadPath(String),
    #[error("lien symbolique refusé : {0}")]
    Symlink(PathBuf),
    #[error("le chemin sort de la racine : {0}")]
    Escape(PathBuf),
    #[error("isolation impossible : {0}")]
    IsolationUnavailable(String),
    #[error("isolation échouée : {0}")]
    Isolation(String),
}
