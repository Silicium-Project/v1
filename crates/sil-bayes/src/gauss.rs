//! Loi normale à variance inconnue (Normal-Gamma, ADR 0013 §4) : modèle conjugué pour une mesure
//! continue (débit, latence, vitesse de calcul). Mise à jour séquentielle, en entiers fixes.
//!
//! Limite reconnue : la vraie prédictive d'un Normal-Gamma est une loi de Student, à queues plus
//! épaisses qu'une gaussienne à faible échantillon (ADR 0013 §4 le signale pour la robustesse aux
//! valeurs aberrantes). Ce module approche la prédictive par une gaussienne construite sur
//! l'espérance de la précision (`alpha/beta`), ce qui sous-estime légèrement l'incertitude quand
//! peu d'observations sont disponibles. Acceptable pour un indice de décision (ADR 0013 §6) ;
//! jamais pour une valeur qui touche l'argent (réservée à ADR 0006/0008/0010/0012).

use crate::fixed::{MICRO, div_micro, mul_micro, phi_micro, sqrt_micro};
use serde::{Deserialize, Serialize};

/// Posterior Normal-Gamma d'une mesure continue, tout en micro (la mesure elle-même, pas son
/// carré : un débit en micro-unités de débit, une latence en micro-secondes...).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NormalGammaPosterior {
    /// Moyenne estimée, en micro.
    pub mu: i64,
    /// Pseudo-compte sur la moyenne (confiance en `mu`), en micro (1 000 000 = un pseudo-point).
    pub kappa: i64,
    /// Forme de la loi Gamma sur la précision, en micro.
    pub alpha: i64,
    /// Taux de la loi Gamma sur la précision, en micro.
    pub beta: i64,
}

impl NormalGammaPosterior {
    /// A priori faiblement informatif centré sur `mu0`, avec une variance de départ `var0` (micro,
    /// dans l'unité au carré de la mesure). `var0` doit être strictement positif.
    pub fn new(mu0: i64, var0: i64) -> Self {
        let var0 = var0.max(1);
        Self {
            mu: mu0,
            kappa: MICRO,
            alpha: MICRO,
            beta: var0,
        }
    }

    /// Nombre d'observations reçues depuis l'a priori.
    pub fn observations(&self) -> i64 {
        self.kappa / MICRO
    }

    /// Ajoute une observation `x` (micro, même unité que `mu0`).
    pub fn observe(&mut self, x: i64) {
        let kappa_next = self.kappa + MICRO;
        let mu_next =
            div_micro(mul_micro(self.kappa, self.mu) + mul_micro(MICRO, x), kappa_next);
        let diff = x - self.mu;
        let diff2 = mul_micro(diff, diff);
        let beta_increment = div_micro(mul_micro(self.kappa, diff2), 2 * kappa_next);
        self.beta += beta_increment;
        self.alpha += MICRO / 2;
        self.kappa = kappa_next;
        self.mu = mu_next;
    }

    /// Moyenne estimée, en micro.
    pub fn mean_micro(&self) -> i64 {
        self.mu
    }

    /// Variance prédictive approchée d'une nouvelle observation (micro², voir la limite en tête de
    /// module).
    fn predictive_variance_micro(&self) -> i64 {
        let base = div_micro(self.beta, self.alpha);
        let factor = div_micro(self.kappa + MICRO, self.kappa);
        mul_micro(base, factor)
    }

    /// `P(prochaine mesure > seuil)`, en micro, sous l'approximation gaussienne de la prédictive.
    pub fn prob_above_micro(&self, threshold_micro: i64) -> i64 {
        let variance = self.predictive_variance_micro().max(1);
        let sigma = sqrt_micro(variance);
        let z = div_micro(self.mu - threshold_micro, sigma);
        phi_micro(z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: i64, want_micro: i64, tol_micro: i64, label: &str) {
        let diff = (got - want_micro).abs();
        assert!(diff <= tol_micro, "{label} : obtenu {got}, attendu {want_micro}, écart {diff}");
    }

    #[test]
    fn observing_values_near_the_prior_mean_barely_moves_it() {
        let mut p = NormalGammaPosterior::new(10 * MICRO, MICRO);
        for _ in 0..5 {
            p.observe(10 * MICRO);
        }
        close(p.mean_micro(), 10 * MICRO, 1_000, "moyenne après 5 observations égales au prior");
    }

    #[test]
    fn observing_higher_values_raises_the_mean_towards_them() {
        let mut p = NormalGammaPosterior::new(0, MICRO);
        for _ in 0..50 {
            p.observe(10 * MICRO);
        }
        // Avec 50 observations contre 1 pseudo-point initial, la moyenne doit être très proche de 10.
        close(p.mean_micro(), 10 * MICRO, 300_000, "moyenne tirée vers les observations");
    }

    #[test]
    fn more_observations_increase_certainty_above_a_lower_bar() {
        let mut few = NormalGammaPosterior::new(10 * MICRO, 4 * MICRO);
        let mut many = few;
        for _ in 0..1 {
            few.observe(10 * MICRO);
        }
        for _ in 0..200 {
            many.observe(10 * MICRO);
        }
        let threshold = 9 * MICRO;
        assert!(
            many.prob_above_micro(threshold) >= few.prob_above_micro(threshold),
            "plus d'observations cohérentes doit augmenter la certitude"
        );
    }

    #[test]
    fn probability_is_centered_at_the_mean() {
        let p = NormalGammaPosterior::new(5 * MICRO, MICRO);
        close(p.prob_above_micro(5 * MICRO), MICRO / 2, 5_000, "P(X > moyenne) ≈ 0,5");
    }

    #[test]
    fn same_observations_always_give_the_same_result() {
        let mut a = NormalGammaPosterior::new(0, MICRO);
        let mut b = NormalGammaPosterior::new(0, MICRO);
        for p in [&mut a, &mut b] {
            p.observe(3 * MICRO);
            p.observe(4 * MICRO);
            p.observe(2 * MICRO);
        }
        assert_eq!(a.prob_above_micro(MICRO), b.prob_above_micro(MICRO));
    }
}
