//! Contrat entre le nœud et un type de calcul (workload).
//!
//! Un workload sait : **découper** une entrée en fragments, **exécuter** un fragment,
//! **vérifier** le résultat d'un autre nœud, et dire **comment** il doit être vérifié
//! ([`VerificationPolicy`]) et avec quelles [`Limits`]. Il ne connaît ni le réseau ni le journal.
//!
//! Les workloads sont enregistrés dans un [`Registry`] passé explicitement (aucun état global,
//! contrairement au `workload::of()` global de V0).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod descriptor;
mod error;
mod evidence;
mod input;
mod output;
mod policy;
mod registry;
mod workload;

pub use descriptor::{limits_of, policy_of};
pub use error::WorkloadError;
pub use evidence::{Evidence, Verdict};
pub use input::{InputReader, MemoryInput};
pub use output::{FragmentSpec, Output};
pub use policy::{Limits, VerificationPolicy};
pub use registry::Registry;
pub use workload::Workload;

#[cfg(test)]
mod tests;
