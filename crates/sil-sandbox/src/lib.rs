//! Exécution bornée d'un programme externe (moteur de rendu, outil d'inférence).
//!
//! **Ce que ce crate garantit** (testé) :
//! - environnement **vide** + liste blanche explicite (le programme ne voit ni jetons ni
//!   variables du nœud) ;
//! - répertoire de travail imposé ;
//! - **délai** : le programme et ses enfants sont tués au-delà de `wall` ;
//! - **sortie plafonnée** : stdout/stderr au-delà du plafond tuent le programme ;
//! - [`Confined`] : chemins relatifs sûrs, liens symboliques et sorties de racine refusés ;
//! - **isolation noyau** si une [`Isolation`] est donnée : sous Linux, limites de ressources,
//!   système de fichiers (Landlock) et réseau (seccomp) ; sous Windows, arbre tué à la fin.
//!
//! Ce qui reste hors garantie sous Windows (mémoire, réseau, fichiers) : voir
//! docs/technical/security/sandbox.md.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod capture;
mod confine;
mod error;
mod isolation;
#[cfg(target_os = "linux")]
mod linux;
mod run;
mod spec;
#[cfg(windows)]
mod windows;

pub use confine::Confined;
pub use error::SandboxError;
pub use isolation::{Isolation, Level, default_launcher};
pub use run::run;
pub use spec::{RunOutcome, SandboxLimits, SandboxSpec};

/// Applique une politique d'isolation au processus courant. Utilisé par le lanceur
/// `sil-sandbox-launch` avant d'exécuter le programme (Linux seulement).
#[cfg(target_os = "linux")]
pub fn apply_isolation(policy: &Isolation) -> Result<(), String> {
    linux::apply(policy)
}

/// Hors Linux, il n'y a pas de lanceur : une isolation noyau ne peut pas être appliquée.
#[cfg(not(target_os = "linux"))]
pub fn apply_isolation(_policy: &Isolation) -> Result<(), String> {
    Err("lanceur d'isolation : Linux seulement".into())
}

#[cfg(test)]
mod tests;
