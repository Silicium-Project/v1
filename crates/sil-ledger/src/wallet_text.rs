//! Texte que le requester signe avec son wallet.
//!
//! Phantom refuse de signer des octets binaires qui ressemblent à une transaction Solana. Ce
//! texte est lisible, en ASCII, et le wallet l'affiche tel quel. Chaque champ a une place fixe :
//! le vérificateur reconstruit exactement le même texte à partir de la structure, donc toute
//! différence de champ ou de format rend la signature invalide.

use crate::JobSpec;
use sil_types::{Hash32, JobId, PublicKey, Signature};

/// Texte signé pour la spec d'un job (le requester signe ce texte, sans préfixe de domaine).
pub fn spec_text(spec: &JobSpec) -> String {
    [
        format!("Silicium JobSpec v{}", spec.version),
        format!("Requester: {}", spec.requester.to_hex()),
        format!("Nonce: {}", spec.nonce),
        format!("Workload: {}", spec.workload),
        format!("Input: {}", spec.input.to_hex()),
        format!("Input size: {}", spec.input_size),
        format!("Width: {}", spec.width),
        format!("Height: {}", spec.height),
        format!("Fragments: {}", spec.fragments),
        format!("Verifications: {}", spec.verifications_required),
        format!("Lease ms: {}", spec.lease_ms),
        format!("Max attempts: {}", spec.max_attempts),
        format!("Reward per unit: {}", spec.reward_per_unit),
        format!(
            "Verifier reward per unit: {}",
            spec.verifier_reward_per_unit
        ),
        format!("Expires ms: {}", spec.expires_at_ms),
    ]
    .join("\n")
}

/// Texte signé par le requester pour annuler son job (événement `Cancel`, phase 4 de l'annulation).
pub fn cancel_text(
    author: &PublicKey,
    job: &JobId,
    sequence: u64,
    previous: &Hash32,
    issued_at_ms: u64,
) -> String {
    [
        "Silicium Event v1".to_string(),
        format!("Author: {}", author.to_hex()),
        format!("Job: {}", job.to_hex()),
        format!("Sequence: {sequence}"),
        format!("Previous: {}", previous.to_hex()),
        format!("Issued ms: {issued_at_ms}"),
        "Body: Cancel".to_string(),
    ]
    .join("\n")
}

/// Texte signé par le requester pour l'événement `JobPublished` de son chaîne.
/// Le contenu de la spec est lié par `spec_signature` : elle signe la spec entière.
pub fn job_published_text(
    author: &PublicKey,
    job: &JobId,
    sequence: u64,
    previous: &Hash32,
    issued_at_ms: u64,
    spec_signature: &Signature,
) -> String {
    [
        "Silicium Event v1".to_string(),
        format!("Author: {}", author.to_hex()),
        format!("Job: {}", job.to_hex()),
        format!("Sequence: {sequence}"),
        format!("Previous: {}", previous.to_hex()),
        format!("Issued ms: {issued_at_ms}"),
        "Body: JobPublished".to_string(),
        format!("Spec signature: {}", spec_signature.to_hex()),
    ]
    .join("\n")
}
