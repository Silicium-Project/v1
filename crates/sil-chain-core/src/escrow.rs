use crate::{EscrowTerms, RuleError};
use serde::{Deserialize, Serialize};
use sil_types::{Hash32, JobId, PublicKey};
use std::collections::{BTreeMap, BTreeSet};

/// Règlement d'un fragment, tel qu'enregistré.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettledFragment {
    pub attempt: u32,
    pub result: Hash32,
    pub claimant: PublicKey,
    pub approvers: Vec<PublicKey>,
}

/// Compte d'escrow d'un job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Escrow {
    pub job: JobId,
    pub terms: EscrowTerms,
    pub(crate) deposited: u64,
    pub(crate) paid: u64,
    pub(crate) refunded: u64,
    pub(crate) closed: bool,
    pub(crate) settled: BTreeMap<u32, SettledFragment>,
    /// Approbateurs déjà pénalisés, par fragment : une faute ne se punit qu'une fois.
    pub(crate) penalized: BTreeSet<(u32, PublicKey)>,
}

impl Escrow {
    /// Ouvre l'escrow. `signer` doit être le requester, et le dépôt couvrir tous les paiements.
    pub fn open(
        job: JobId,
        terms: EscrowTerms,
        deposit: u64,
        signer: &PublicKey,
    ) -> Result<Self, RuleError> {
        terms.validate()?;
        if *signer != terms.requester {
            return Err(RuleError::NotRequester);
        }
        let need = terms.max_payout()?;
        if deposit < need {
            return Err(RuleError::DepositTooLow {
                have: deposit,
                need,
            });
        }
        Ok(Self {
            job,
            terms,
            deposited: deposit,
            paid: 0,
            refunded: 0,
            closed: false,
            settled: BTreeMap::new(),
            penalized: BTreeSet::new(),
        })
    }

    /// Ajoute des fonds (par exemple pour un job prolongé). Seul le requester, tant que
    /// l'escrow n'est pas fermé.
    pub fn top_up(&mut self, signer: &PublicKey, amount: u64) -> Result<(), RuleError> {
        if *signer != self.terms.requester {
            return Err(RuleError::NotRequester);
        }
        if self.closed {
            return Err(RuleError::Refunded);
        }
        self.deposited = self
            .deposited
            .checked_add(amount)
            .ok_or(RuleError::Overflow)?;
        Ok(())
    }

    /// Fonds encore disponibles dans l'escrow.
    pub fn balance(&self) -> u64 {
        self.deposited - self.paid - self.refunded
    }

    pub fn deposited(&self) -> u64 {
        self.deposited
    }

    pub fn paid(&self) -> u64 {
        self.paid
    }

    pub fn refunded(&self) -> u64 {
        self.refunded
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn settled(&self, fragment: u32) -> Option<&SettledFragment> {
        self.settled.get(&fragment)
    }

    pub fn settled_count(&self) -> usize {
        self.settled.len()
    }
}
