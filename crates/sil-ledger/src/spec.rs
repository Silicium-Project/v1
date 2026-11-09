use crate::LedgerError;
use crate::wallet_text::spec_text;
use serde::{Deserialize, Serialize};
use sil_codec::encode;
use sil_crypto::{Domain, SigningIdentity, sha256_domain, verify_raw};
use sil_types::{Hash32, JobId, PublicKey, Signature};

pub const SPEC_VERSION: u8 = 2;
pub const MAX_FRAGMENTS: u32 = 1024;
pub const MAX_VERIFICATIONS: u32 = 16;
pub const MAX_SPEC_ATTEMPTS: u32 = 16;
const MAX_WORKLOAD_LEN: usize = 80;

/// Description d'un job, signée par le requester. `job_id = H(spec)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSpec {
    pub version: u8,
    pub requester: PublicKey,
    pub nonce: u64,
    /// Identifiant de workload (`a-z0-9/._-`).
    pub workload: String,
    /// Hash du contenu d'entrée.
    pub input: Hash32,
    /// Taille du contenu d'entrée, en octets. Le workload en déduit les unités de travail.
    pub input_size: u64,
    /// Largeur de l'image en pixels (`rendu/raytracer`). 0 pour un workload sans image.
    pub width: u32,
    /// Hauteur de l'image en pixels (`rendu/raytracer`). 0 pour un workload sans image.
    pub height: u32,
    pub fragments: u32,
    /// Approbations indépendantes nécessaires pour vérifier un résultat.
    pub verifications_required: u32,
    pub lease_ms: u64,
    pub max_attempts: u32,
    /// Tarif d'une unité de travail, versé au worker qui réalise le fragment.
    pub reward_per_unit: u64,
    /// Tarif d'une unité de travail, versé à chaque vérificateur.
    pub verifier_reward_per_unit: u64,
    pub expires_at_ms: u64,
}

/// Spec accompagnée de la signature du requester.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedJobSpec {
    pub spec: JobSpec,
    pub signature: Signature,
}

fn bad(reason: &'static str) -> LedgerError {
    LedgerError::InvalidSpec(reason)
}

impl JobSpec {
    /// Contrôles structurels (bornes et règles du job).
    pub fn validate(&self) -> Result<(), LedgerError> {
        if self.version != SPEC_VERSION {
            return Err(LedgerError::BadVersion);
        }
        let workload_ok = !self.workload.is_empty()
            && self.workload.len() <= MAX_WORKLOAD_LEN
            && self
                .workload
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"/._-".contains(&b));
        if !workload_ok {
            return Err(bad("workload"));
        }
        if self.fragments == 0 || self.fragments > MAX_FRAGMENTS {
            return Err(bad("fragments"));
        }
        if self.verifications_required == 0 || self.verifications_required > MAX_VERIFICATIONS {
            return Err(bad("verifications_required"));
        }
        if self.lease_ms == 0 {
            return Err(bad("lease_ms"));
        }
        if self.max_attempts == 0 || self.max_attempts > MAX_SPEC_ATTEMPTS {
            return Err(bad("max_attempts"));
        }
        Ok(())
    }

    /// Octets CBOR canoniques de la spec : ce que le requester signe (domaine `JobSpec`).
    pub fn bytes(&self) -> Result<Vec<u8>, LedgerError> {
        Ok(encode(self)?)
    }

    /// Identifiant du job.
    pub fn job_id(&self) -> Result<JobId, LedgerError> {
        let hash = sha256_domain(Domain::JobId, &self.bytes()?);
        Ok(JobId::from_bytes(*hash.as_bytes()))
    }
}

impl SignedJobSpec {
    /// Valide et signe. L'identité doit être celle du requester.
    pub fn sign(identity: &SigningIdentity, spec: JobSpec) -> Result<Self, LedgerError> {
        spec.validate()?;
        if identity.public() != spec.requester {
            return Err(LedgerError::NotAuthorized("le requester doit signer"));
        }
        let signature = identity.sign_raw(spec_text(&spec).as_bytes());
        Ok(Self { spec, signature })
    }

    /// Valide la spec et la signature ; retourne l'identifiant du job.
    pub fn verify(&self) -> Result<JobId, LedgerError> {
        self.spec.validate()?;
        verify_raw(
            &self.spec.requester,
            spec_text(&self.spec).as_bytes(),
            &self.signature,
        )
        .map_err(|_| LedgerError::BadSignature)?;
        self.spec.job_id()
    }
}
