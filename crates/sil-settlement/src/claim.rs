use crate::SettlementError;
use serde::{Deserialize, Serialize};
use sil_chain_core::{SettleArgs, SettledFragment};
use sil_crypto::{Domain, SigningIdentity, sha256_domain, verify};
use sil_ledger::Finalized;
use sil_types::{Hash32, JobId, PublicKey, Signature};

/// Identifiant d'un règlement : hash de ses paramètres. Le même règlement a toujours le
/// même identifiant, sur tous les nœuds.
pub type IntentId = Hash32;

/// Plus grand nombre de signatures conservées (une par approbateur, au plus `MAX_APPROVERS`).
const MAX_SIGNATURES: usize = 16;

/// Ce que les approbateurs autorisent à payer pour un fragment (ADR 0007).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementClaim {
    pub job: JobId,
    pub fragment: u32,
    pub attempt: u32,
    pub result: Hash32,
    pub claimant: PublicKey,
    /// Exactement `verifications_required` vérificateurs, triés par clé.
    pub approvers: Vec<PublicKey>,
}

impl SettlementClaim {
    /// Construit le règlement d'un fragment finalisé. `None` s'il y a moins d'approbateurs que
    /// `required` (rien à payer tant que le journal n'a pas assez d'approbations).
    pub fn from_finalized(
        job: &JobId,
        fragment: u32,
        finalized: &Finalized,
        required: u32,
    ) -> Option<Self> {
        let mut approvers = finalized.approvers.clone();
        approvers.sort();
        approvers.dedup();
        approvers.truncate(required as usize);
        (approvers.len() == required as usize).then_some(Self {
            job: *job,
            fragment,
            attempt: finalized.attempt,
            result: finalized.result,
            claimant: finalized.claimant,
            approvers,
        })
    }

    /// Message signé par chaque approbateur (sans préfixe de domaine), défini dans `sil-chain-core`.
    /// Ne contient pas la liste des approbateurs : elle n'est connue qu'à la finalisation.
    pub fn bytes(&self) -> Vec<u8> {
        sil_chain_core::wire::settlement_message(&self.job, &self.args())
    }

    pub fn id(&self) -> IntentId {
        sha256_domain(Domain::Settlement, &self.bytes())
    }

    pub fn sign(&self, identity: &SigningIdentity) -> Signature {
        identity.sign(Domain::Settlement, &self.bytes())
    }

    /// Paramètres pour les règles de l'escrow.
    pub fn args(&self) -> SettleArgs {
        SettleArgs {
            fragment: self.fragment,
            attempt: self.attempt,
            result: self.result,
            claimant: self.claimant,
            approvers: self.approvers.clone(),
        }
    }

    /// Ce règlement est-il exactement celui enregistré sur la chaîne ?
    pub fn matches(&self, settled: &SettledFragment) -> bool {
        self.attempt == settled.attempt
            && self.result == settled.result
            && self.claimant == settled.claimant
            && self.approvers == settled.approvers
    }
}

/// Un règlement et les signatures des approbateurs qui l'autorisent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authorization {
    pub claim: SettlementClaim,
    pub signatures: Vec<(PublicKey, Signature)>,
}

impl Authorization {
    pub fn new(claim: SettlementClaim) -> Self {
        Self {
            claim,
            signatures: Vec::new(),
        }
    }

    /// Ajoute une signature après l'avoir vérifiée. Retourne `Ok(false)` si le signataire est
    /// déjà présent.
    pub fn add(
        &mut self,
        signer: PublicKey,
        signature: Signature,
    ) -> Result<bool, SettlementError> {
        verify(&signer, Domain::Settlement, &self.claim.bytes(), &signature)
            .map_err(|_| SettlementError::BadSignature)?;
        if self.signatures.iter().any(|(known, _)| *known == signer)
            || self.signatures.len() >= MAX_SIGNATURES
        {
            return Ok(false);
        }
        self.signatures.push((signer, signature));
        Ok(true)
    }

    /// Approbateurs **distincts** dont la signature est valide.
    pub fn approver_signers(&self) -> Vec<PublicKey> {
        let message = self.claim.bytes();
        let mut signers: Vec<PublicKey> = self
            .signatures
            .iter()
            .filter(|(signer, _)| self.claim.approvers.contains(signer))
            .filter(|(signer, signature)| {
                verify(signer, Domain::Settlement, &message, signature).is_ok()
            })
            .map(|(signer, _)| *signer)
            .collect();
        signers.sort();
        signers.dedup();
        signers
    }
}
