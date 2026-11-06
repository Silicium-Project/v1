use crate::{
    Escrow, RuleError, SettledFragment,
    terms::{MAX_APPROVERS, SETTLEMENT_FEE},
};
use serde::{Deserialize, Serialize};
use sil_types::{Hash32, PublicKey};
use std::collections::BTreeSet;

/// Un transfert à effectuer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payout {
    pub to: PublicKey,
    pub amount: u64,
}

/// Paramètres d'un règlement (ceux que le comité a signés).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettleArgs {
    pub fragment: u32,
    pub attempt: u32,
    pub result: Hash32,
    pub claimant: PublicKey,
    /// Exactement `verifications_required` vérificateurs distincts, différents du claimant.
    pub approvers: Vec<PublicKey>,
}

impl SettleArgs {
    fn matches(&self, settled: &SettledFragment) -> bool {
        self.attempt == settled.attempt
            && self.result == settled.result
            && self.claimant == settled.claimant
            && self.approvers == settled.approvers
    }
}

impl Escrow {
    /// Règle un fragment. `signers` : clés dont la signature du message de règlement a été
    /// **vérifiée** par l'appelant (précompilé Ed25519 sur la chaîne). `submitter` : celui qui
    /// soumet la transaction ; il reçoit les frais de règlement. Retourne les transferts.
    pub fn settle(
        &mut self,
        args: &SettleArgs,
        signers: &[PublicKey],
        submitter: &PublicKey,
    ) -> Result<Vec<Payout>, RuleError> {
        if self.closed {
            return Err(RuleError::Refunded);
        }
        if args.fragment >= self.terms.fragments {
            return Err(RuleError::UnknownFragment);
        }
        if let Some(existing) = self.settled.get(&args.fragment) {
            return Err(RuleError::AlreadySettled {
                same: args.matches(existing),
            });
        }
        if !(1..=16).contains(&args.attempt) {
            return Err(RuleError::BadAttempt);
        }
        self.check_approvers(args)?;
        self.check_authorization(signers)?;

        let units = self.terms.fragment_units(args.fragment)?;
        let claimant_amount = units
            .checked_mul(self.terms.reward_per_unit)
            .ok_or(RuleError::Overflow)?;
        let verifier_amount = units
            .checked_mul(self.terms.verifier_reward_per_unit)
            .ok_or(RuleError::Overflow)?;
        let total = self.terms.fragment_budget(args.fragment)?;
        if total > self.balance() {
            return Err(RuleError::Insufficient);
        }
        let mut payouts = Vec::new();
        if claimant_amount > 0 {
            payouts.push(Payout {
                to: args.claimant,
                amount: claimant_amount,
            });
        }
        if verifier_amount > 0 {
            payouts.extend(args.approvers.iter().map(|to| Payout {
                to: *to,
                amount: verifier_amount,
            }));
        }
        payouts.push(Payout {
            to: *submitter,
            amount: SETTLEMENT_FEE,
        });
        self.paid = self.paid.checked_add(total).ok_or(RuleError::Overflow)?;
        self.settled.insert(
            args.fragment,
            SettledFragment {
                attempt: args.attempt,
                result: args.result,
                claimant: args.claimant,
                approvers: args.approvers.clone(),
            },
        );
        Ok(payouts)
    }

    fn check_approvers(&self, args: &SettleArgs) -> Result<(), RuleError> {
        let required = self.terms.verifications_required as usize;
        if args.approvers.len() != required || required > MAX_APPROVERS {
            return Err(RuleError::BadApprovers("nombre"));
        }
        let distinct: BTreeSet<_> = args.approvers.iter().collect();
        if distinct.len() != required {
            return Err(RuleError::BadApprovers("doublons"));
        }
        if args.approvers.contains(&args.claimant) {
            return Err(RuleError::BadApprovers("le claimant ne se vérifie pas"));
        }
        Ok(())
    }

    fn check_authorization(&self, signers: &[PublicKey]) -> Result<(), RuleError> {
        let members: BTreeSet<_> = signers
            .iter()
            .filter(|signer| self.terms.committee.contains(signer))
            .collect();
        if members.len() < self.terms.threshold as usize {
            return Err(RuleError::NotAuthorized {
                have: members.len(),
                need: self.terms.threshold,
            });
        }
        Ok(())
    }
}
