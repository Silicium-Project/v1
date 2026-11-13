//! Loi Beta-Bernoulli (ADR 0013 §4) : modèle conjugué d'un taux (succès, fausse vérification,
//! rétention après audit). Exacte, bornée à `[0, 1]` par construction — préférée à une
//! approximation gaussienne quand l'échantillon est petit ou le taux proche de 0 ou 1 (comparaison
//! tracée en séance). `P(taux > seuil)` est la fonction bêta incomplète régularisée, recalculée en
//! entiers fixes : pas de bibliothèque statistique, pas de flottant système (`fixed` module).
//!
//! Précision : ce module vise un indice de décision (ADR 0013 §6 : un prior, pas une valeur de
//! paiement), pas une précision statistique de référence. La tolérance attendue est de l'ordre du
//! millième (0,1 %), pas du millionième.

use crate::fixed::{MICRO, div_micro, exp_micro, ln_micro, mul_micro};
use serde::{Deserialize, Serialize};

/// Posterior `Beta(alpha, beta)` d'un taux. `alpha`/`beta` sont les pseudo-comptes : a priori plus
/// observations. Un a priori `Beta(1, 1)` (uniforme) est le point de départ usuel (`UNIFORM`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BetaPosterior {
    pub alpha: u64,
    pub beta: u64,
}

impl BetaPosterior {
    /// A priori uniforme : `Beta(1, 1)`, aucune observation.
    pub const UNIFORM: Self = Self { alpha: 1, beta: 1 };

    /// A priori `Beta(alpha0, beta0)` ; chaque borne est portée à 1 au minimum (une loi Beta
    /// exige des paramètres strictement positifs).
    pub fn new(alpha0: u64, beta0: u64) -> Self {
        Self {
            alpha: alpha0.max(1),
            beta: beta0.max(1),
        }
    }

    /// Une observation de succès.
    pub fn observe_success(&mut self) {
        self.alpha = self.alpha.saturating_add(1);
    }

    /// Une observation d'échec.
    pub fn observe_failure(&mut self) {
        self.beta = self.beta.saturating_add(1);
    }

    /// Nombre d'observations reçues depuis l'a priori `Beta(1, 1)`. Approximatif pour un autre
    /// a priori : seul le nombre total de pseudo-comptes au-delà de 2 compte.
    pub fn observations(&self) -> u64 {
        (self.alpha + self.beta).saturating_sub(2)
    }

    /// Moyenne du posterior, en micro : `alpha / (alpha + beta)`.
    pub fn mean_micro(&self) -> i64 {
        let total = self.alpha + self.beta;
        ((u128::from(self.alpha) * MICRO as u128) / u128::from(total)) as i64
    }

    /// `P(taux > seuil)`, en micro (0 à 1 000 000). `seuil_micro` est pris dans `[0, 1 000 000]`.
    pub fn prob_above_micro(&self, threshold_micro: i64) -> i64 {
        let x = threshold_micro.clamp(0, MICRO);
        if x <= 0 {
            return MICRO;
        }
        if x >= MICRO {
            return 0;
        }
        let a = (self.alpha * MICRO as u64) as i64;
        let b = (self.beta * MICRO as u64) as i64;
        let i_x = regularized_incomplete_beta(a, b, x);
        (MICRO - i_x).clamp(0, MICRO)
    }
}

/// `ln Γ(z)`, `z` en micro (`z > 0`), résultat en micro. Décale `z` au-delà de 8 (bonne précision
/// de Stirling) par la relation de récurrence `Γ(z) = Γ(z+1)/z`, puis applique la série de
/// Stirling. Le décalage est borné (au plus 8 pas) : toujours déterministe et rapide.
fn lgamma_micro(z_micro: i64) -> i64 {
    assert!(z_micro > 0, "lgamma n'est défini que pour z > 0");
    let mut z = z_micro;
    let mut correction: i64 = 0;
    while z < 8 * MICRO {
        correction += ln_micro(z);
        z += MICRO;
    }
    let half = MICRO / 2;
    let z_minus_half = z - half;
    let ln_z = ln_micro(z);
    let term1 = mul_micro(z_minus_half, ln_z);
    // ln(2π)/2 ≈ 0,918938533205, en micro.
    const HALF_LN_2PI: i64 = 918_939;
    let twelve_z = mul_micro(12 * MICRO, z);
    let inv_12z = div_micro(MICRO, twelve_z);
    let z3 = mul_micro(mul_micro(z, z), z);
    let inv_360z3 = div_micro(MICRO, mul_micro(360 * MICRO, z3));
    let z5 = mul_micro(mul_micro(z3, z), z);
    let inv_1260z5 = div_micro(MICRO, mul_micro(1_260 * MICRO, z5));
    let stirling = term1 - z + HALF_LN_2PI + inv_12z - inv_360z3 + inv_1260z5;
    stirling - correction
}

/// Fraction continue de Lentz pour la fonction bêta incomplète (Numerical Recipes, `betacf`),
/// entièrement en micro. Nombre fixe d'itérations : déterministe, pas de sortie anticipée sur un
/// critère de convergence qui dépendrait de l'arrondi.
fn beta_continued_fraction(a: i64, b: i64, x: i64) -> i64 {
    const TINY: i64 = 1; // garde-fou contre une division par zéro, pas une tolérance numérique.
    const ITERATIONS: i64 = 60;
    let qab = a + b;
    let qap = a + MICRO;
    let qam = a - MICRO;
    let mut c = MICRO;
    let mut d = MICRO - div_micro(mul_micro(qab, x), qap);
    if d.abs() < TINY {
        d = TINY;
    }
    d = div_micro(MICRO, d);
    let mut h = d;
    for m in 1..=ITERATIONS {
        let m2 = MICRO * (2 * m);
        let m_micro = MICRO * m;
        let num1 = mul_micro(mul_micro(m_micro, b - m_micro), x);
        let den1 = mul_micro(qam + m2, a + m2);
        let aa1 = div_micro(num1, den1);
        d = MICRO + mul_micro(aa1, d);
        if d.abs() < TINY {
            d = TINY;
        }
        c = MICRO + div_micro(aa1, c);
        if c.abs() < TINY {
            c = TINY;
        }
        d = div_micro(MICRO, d);
        h = mul_micro(h, mul_micro(d, c));
        let num2 = mul_micro(mul_micro(-(a + m_micro), qab + m_micro), x);
        let den2 = mul_micro(a + m2, qap + m2);
        let aa2 = div_micro(num2, den2);
        d = MICRO + mul_micro(aa2, d);
        if d.abs() < TINY {
            d = TINY;
        }
        c = MICRO + div_micro(aa2, c);
        if c.abs() < TINY {
            c = TINY;
        }
        d = div_micro(MICRO, d);
        let del = mul_micro(d, c);
        h = mul_micro(h, del);
    }
    h
}

/// Fonction bêta incomplète régularisée `I_x(a, b)`, `a`, `b`, `x` en micro (`a, b > 0`,
/// `0 < x < 1`).
fn regularized_incomplete_beta(a: i64, b: i64, x: i64) -> i64 {
    let ln_bt = lgamma_micro(a + b) - lgamma_micro(a) - lgamma_micro(b)
        + mul_micro(a, ln_micro(x))
        + mul_micro(b, ln_micro(MICRO - x));
    let bt = exp_micro(ln_bt);
    // Région de bonne convergence de la fraction continue : x < (a+1)/(a+b+2).
    let pivot = div_micro(a + MICRO, a + b + 2 * MICRO);
    if x < pivot {
        div_micro(mul_micro(bt, beta_continued_fraction(a, b, x)), a)
    } else {
        let complement = div_micro(
            mul_micro(bt, beta_continued_fraction(b, a, MICRO - x)),
            b,
        );
        MICRO - complement
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: i64 = 2_000; // 0,2 % : tolérance d'un indice, pas d'une valeur de paiement.

    fn close(got: i64, want_micro: i64, label: &str) {
        let diff = (got - want_micro).abs();
        assert!(
            diff <= TOL,
            "{label} : obtenu {got}, attendu {want_micro}, écart {diff}"
        );
    }

    #[test]
    fn a_uniform_prior_gives_a_linear_survival() {
        // Beta(1,1) : P(X > x) = 1 - x, exactement.
        let p = BetaPosterior::UNIFORM;
        for x in [0, 100_000, 300_000, 500_000, 700_000, 999_000] {
            close(p.prob_above_micro(x), MICRO - x, "Beta(1,1)");
        }
    }

    #[test]
    fn beta_two_one_matches_its_closed_form() {
        // Beta(2,1) : densité 2x sur [0,1], CDF x², donc P(X > x) = 1 - x².
        let p = BetaPosterior::new(2, 1);
        for x in [100_000i64, 300_000, 500_000, 800_000] {
            let x2 = mul_micro(x, x);
            close(p.prob_above_micro(x), MICRO - x2, "Beta(2,1)");
        }
    }

    #[test]
    fn a_symmetric_beta_splits_evenly_at_one_half() {
        // Beta(a,a) est symétrique autour de 0,5 : P(X > 0,5) = 0,5, quel que soit a.
        for a in [1u64, 2, 3, 5, 10, 30, 80] {
            let p = BetaPosterior::new(a, a);
            close(p.prob_above_micro(MICRO / 2), MICRO / 2, "Beta(a,a) symétrique");
        }
    }

    #[test]
    fn more_successes_raise_the_mean_and_the_certainty() {
        let mut p = BetaPosterior::UNIFORM;
        for _ in 0..19 {
            p.observe_success();
        }
        p.observe_failure();
        // 19 succès, 1 échec : moyenne ≈ 20/22 ≈ 0,909.
        close(p.mean_micro(), 909_091, "moyenne après 19 succès, 1 échec");
        assert!(p.prob_above_micro(500_000) > 990_000, "quasi certain au-dessus de 0,5");
        assert!(p.prob_above_micro(990_000) < 500_000, "pas certain au-dessus de 0,99");
    }

    #[test]
    fn probability_is_monotonic_in_the_threshold() {
        let p = BetaPosterior::new(7, 3);
        let mut previous = MICRO + 1;
        let mut x = 0;
        while x <= MICRO {
            let value = p.prob_above_micro(x);
            assert!(value <= previous, "doit décroître avec le seuil");
            previous = value;
            x += 50_000;
        }
    }

    #[test]
    fn same_observations_always_give_the_same_result() {
        let mut a = BetaPosterior::UNIFORM;
        let mut b = BetaPosterior::UNIFORM;
        for p in [&mut a, &mut b] {
            p.observe_success();
            p.observe_success();
            p.observe_failure();
        }
        assert_eq!(a.prob_above_micro(600_000), b.prob_above_micro(600_000));
    }
}
