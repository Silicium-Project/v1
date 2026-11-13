//! Attestations et certificats de finalisation (ADR 0002).
//!
//! Une [`Attestation`] est un verdict signé d'un vérificateur sur un résultat précis.
//! Un rejet porte une référence de preuve (`evidence`) : pas de rejet "nu".
//! Un [`Certificate`] regroupe ≥ k approbations et se revalide sans faire confiance
//! à celui qui le présente (coordinateur, chaîne, autre pair).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod attestation;
mod certificate;
mod error;
mod tolerance;

pub use attestation::{Attestation, AttestationBody};
pub use certificate::{CertRules, Certificate, MAX_APPROVALS};
pub use error::AttestError;
pub use tolerance::{ToleranceAttestation, ToleranceBody, ToleranceCertificate, ToleranceRules};

#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod tests;
