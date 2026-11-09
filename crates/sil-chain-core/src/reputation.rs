//! Réputation de vérificateur, tenue on-chain (gate au règlement, ADR 0006, 0007, 0008).
//!
//! Échelle et seuil repris de V0 (`chain/solana/programs/solana-layer/src/state/reputation.rs` :
//! départ 5 000, éligibilité à 2 500, bornes 0 à 10 000). Le crédit et la pénalité sont propres à V1.
//! Le crédit vient d'un règlement approuvé. La pénalité vient du requester (ADR 0008).
//!
//! Un vérificateur sans compte, ou sous le seuil, ne peut pas être payé par un règlement.

/// Plancher du score.
pub const VERIFIER_MIN: i64 = 0;
/// Plafond du score.
pub const VERIFIER_MAX: i64 = 10_000;
/// Score d'un vérificateur à l'enregistrement.
pub const VERIFIER_START: i64 = 5_000;
/// Seuil d'éligibilité : en dessous, aucun règlement ne paie ce vérificateur.
pub const VERIFIER_ELIGIBLE: i64 = 2_500;
/// Crédit pour chaque règlement où le vérificateur a approuvé le résultat.
pub const VERIFIED_STEP: i64 = 100;
/// Pénalité pour une approbation d'un résultat faux, signée par le requester (ADR 0008).
pub const PENALTY_STEP: i64 = 1_000;

/// Score après un règlement approuvé.
pub fn credited(score: i64) -> i64 {
    (score + VERIFIED_STEP).clamp(VERIFIER_MIN, VERIFIER_MAX)
}

/// Score après une pénalité.
pub fn penalized(score: i64) -> i64 {
    (score - PENALTY_STEP).clamp(VERIFIER_MIN, VERIFIER_MAX)
}

/// Vrai si un vérificateur de ce score peut être payé.
pub fn is_eligible(score: i64) -> bool {
    score >= VERIFIER_ELIGIBLE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_verifier_is_eligible_and_three_penalties_are_not() {
        assert!(is_eligible(VERIFIER_START));
        let twice = penalized(penalized(VERIFIER_START));
        let thrice = penalized(twice);
        assert!(is_eligible(twice));
        assert!(!is_eligible(thrice));
    }

    #[test]
    fn the_score_is_capped_and_floored() {
        assert_eq!(credited(VERIFIER_MAX), VERIFIER_MAX);
        assert_eq!(penalized(VERIFIER_MIN), VERIFIER_MIN);
        assert_eq!(penalized(500), VERIFIER_MIN);
        assert_eq!(credited(VERIFIER_START), VERIFIER_START + VERIFIED_STEP);
    }

    #[test]
    fn eligibility_starts_exactly_at_the_threshold() {
        assert!(is_eligible(VERIFIER_ELIGIBLE));
        assert!(!is_eligible(VERIFIER_ELIGIBLE - 1));
    }
}
