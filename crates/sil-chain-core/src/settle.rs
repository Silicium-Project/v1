use crate::{
    Escrow, RuleError, SettledFragment,
    reputation::{self, VERIFIER_ELIGIBLE},
    reserve,
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

/// Résultat d'un règlement accepté : les transferts, et le nouveau score de réputation de chaque
/// vérificateur approuvé (à écrire dans son compte par le programme).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settlement {
    pub payouts: Vec<Payout>,
    pub credited: Vec<(PublicKey, i64)>,
    /// Part de la récompense de chaque vérificateur gardée en réserve on-chain (ADR 0010). Elle
    /// n'est pas dans `payouts` : le programme la verse au compte du vérificateur.
    pub reserved: Vec<(PublicKey, u64)>,
}

/// Paramètres d'un règlement (ceux que les approbateurs ont signés).
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
    /// soumet la transaction ; il reçoit les frais de règlement. `standing` : score de réputation
    /// de chaque approbateur, dans l'ordre de `args.approvers` (`None` : pas de compte). Un
    /// approbateur sans compte ou sous le seuil refuse le règlement.
    pub fn settle(
        &mut self,
        args: &SettleArgs,
        signers: &[PublicKey],
        submitter: &PublicKey,
        standing: &[Option<i64>],
    ) -> Result<Settlement, RuleError> {
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
        check_signed(&args.approvers, signers)?;
        let credited = check_standing(&args.approvers, standing)?;

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
        // Une part de la récompense reste en réserve au nom du vérificateur (ADR 0010). Le budget
        // du fragment ne change pas : seule la destination d'une partie de la somme.
        let (verifier_paid, verifier_held) = reserve::split(verifier_amount);
        let mut reserved = Vec::new();
        if verifier_amount > 0 {
            for to in &args.approvers {
                if verifier_paid > 0 {
                    payouts.push(Payout {
                        to: *to,
                        amount: verifier_paid,
                    });
                }
                if verifier_held > 0 {
                    reserved.push((*to, verifier_held));
                }
            }
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
        Ok(Settlement {
            payouts,
            credited,
            reserved,
        })
    }

    pub(crate) fn check_approvers(&self, args: &SettleArgs) -> Result<(), RuleError> {
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
}

/// Chaque approbateur a signé le règlement (ADR 0007) : il n'y a plus de comité. `signers` sont les
/// clés dont la signature `Domain::Settlement` a été vérifiée sur le message du règlement.
fn check_signed(approvers: &[PublicKey], signers: &[PublicKey]) -> Result<(), RuleError> {
    let signed = approvers.iter().filter(|a| signers.contains(a)).count();
    if signed < approvers.len() {
        return Err(RuleError::NotAuthorized {
            have: signed,
            need: u32::try_from(approvers.len()).unwrap_or(u32::MAX),
        });
    }
    Ok(())
}

/// Chaque approbateur doit avoir un score éligible. Retourne son nouveau score, crédité.
fn check_standing(
    approvers: &[PublicKey],
    standing: &[Option<i64>],
) -> Result<Vec<(PublicKey, i64)>, RuleError> {
    if standing.len() != approvers.len() {
        return Err(RuleError::BadApprovers("réputation absente"));
    }
    approvers
        .iter()
        .zip(standing)
        .map(|(approver, score)| match score {
            Some(score) if reputation::is_eligible(*score) => {
                Ok((*approver, reputation::credited(*score)))
            }
            other => Err(RuleError::IneligibleVerifier {
                have: *other,
                need: VERIFIER_ELIGIBLE,
            }),
        })
        .collect()
}
