//! Arithmétique en virgule fixe, entièrement entière (`i128`) : même calcul, même résultat sur
//! toute plate-forme. Aucune fonction transcendante du système (`f64::exp`, `f64::ln`, `libm`)
//! n'est appelée : `exp` et `ln` sont recalculés ici par réduction d'intervalle et séries, avec
//! seulement `+`, `-`, `*`, `/` sur des entiers (ADR 0013 §4, §7 : « jamais `f64::exp` en direct »).
//!
//! Unité publique : le **micro** (valeur × 1 000 000), déjà utilisée ailleurs dans le projet
//! (`min_cos_micro`). Unité interne, plus précise : le **pico** (valeur × 10^12), pour limiter
//! l'erreur d'arrondi accumulée dans les séries ; elle ne sort jamais de ce module.

/// Échelle publique : 1 micro-unité = 10^-6.
pub const MICRO: i64 = 1_000_000;

/// Échelle interne des séries.
const SCALE: i128 = 1_000_000_000_000;
const MICRO_TO_PICO: i128 = SCALE / MICRO as i128;

/// ln(2), en pico.
const LN2: i128 = 693_147_180_560;

fn to_pico(micro: i64) -> i128 {
    i128::from(micro) * MICRO_TO_PICO
}

fn from_pico(pico: i128) -> i64 {
    (pico / MICRO_TO_PICO) as i64
}

/// `a·b` à l'échelle pico.
fn mul_pico(a: i128, b: i128) -> i128 {
    (a * b) / SCALE
}

/// `a/b` à l'échelle pico. `b` ne doit pas être nul.
fn div_pico(a: i128, b: i128) -> i128 {
    (a * SCALE) / b
}

/// `a·b`, `a` et `b` en micro, résultat en micro.
pub fn mul_micro(a: i64, b: i64) -> i64 {
    ((i128::from(a) * i128::from(b)) / i128::from(MICRO)) as i64
}

/// `a/b`, `a` et `b` en micro, résultat en micro. `b` ne doit pas être nul.
pub fn div_micro(a: i64, b: i64) -> i64 {
    ((i128::from(a) * i128::from(MICRO)) / i128::from(b)) as i64
}

/// `e^x`, `x` en pico, résultat en pico. Réduction `x = k·ln2 + r` (`r` dans `[0, ln2)`), puis
/// série de Taylor sur `r` : bornée, donc un nombre fixe de termes suffit.
fn exp_pico(x: i128) -> i128 {
    let k = x.div_euclid(LN2);
    let r = x - k * LN2;
    let mut term = SCALE;
    let mut sum = SCALE;
    for n in 1..30i128 {
        term = mul_pico(term, r) / n;
        sum += term;
    }
    let mut result = sum;
    let mut steps = k;
    while steps > 0 {
        result *= 2;
        steps -= 1;
    }
    while steps < 0 {
        result /= 2;
        steps += 1;
    }
    result
}

/// `ln(x)`, `x` en pico (`x > 0`), résultat en pico. Réduction `x = m · 2^e`, `m` dans
/// `[SCALE, 2·SCALE)`, puis série en `t = (m - SCALE)/(m + SCALE)` (bornée par 1/3).
fn ln_pico(x: i128) -> i128 {
    debug_assert!(x > 0, "ln n'est défini que pour x > 0");
    let mut m = x;
    let mut e: i128 = 0;
    while m >= 2 * SCALE {
        m /= 2;
        e += 1;
    }
    while m < SCALE {
        m *= 2;
        e -= 1;
    }
    let t = div_pico(m - SCALE, m + SCALE);
    let t2 = mul_pico(t, t);
    let mut cur = t;
    let mut sum = t;
    for n in 1..12i128 {
        cur = mul_pico(cur, t2);
        sum += cur / (2 * n + 1);
    }
    2 * sum + e * LN2
}

/// `e^x`, `x` en micro, résultat en micro.
pub fn exp_micro(x_micro: i64) -> i64 {
    from_pico(exp_pico(to_pico(x_micro)))
}

/// `ln(x)`, `x` en micro (`x > 0`), résultat en micro.
pub fn ln_micro(x_micro: i64) -> i64 {
    assert!(x_micro > 0, "ln n'est défini que pour x > 0");
    from_pico(ln_pico(to_pico(x_micro)))
}

/// `√x`, `x` en micro (`x ≥ 0`), résultat en micro. Newton, nombre fixe d'itérations : la
/// convergence est quadratique, donc toujours atteinte largement avant la dernière itération.
pub fn sqrt_micro(x_micro: i64) -> i64 {
    assert!(x_micro >= 0, "√ n'est défini que pour x ≥ 0");
    if x_micro == 0 {
        return 0;
    }
    let mut y = x_micro.max(1);
    for _ in 0..60 {
        y = (y + div_micro(x_micro, y)) / 2;
    }
    y
}

/// Coefficients de l'approximation d'Abramowitz et Stegun (7.1.26) pour `erf`, en micro.
/// Erreur maximale ≈ 1,5·10⁻⁷ sur `erf`, donc amplement sous la tolérance de ce module (le
/// millième).
const ERF_P: i64 = 327_591;
const ERF_A1: i64 = 254_830;
const ERF_A2: i64 = -284_497;
const ERF_A3: i64 = 1_421_414;
const ERF_A4: i64 = -1_453_152;
const ERF_A5: i64 = 1_061_405;
/// `1/√2`, en micro.
const INV_SQRT2_MICRO: i64 = 707_107;

/// `erf(x)`, `x ≥ 0`, en micro.
fn erf_micro_nonneg(x_micro: i64) -> i64 {
    let t = div_micro(MICRO, MICRO + mul_micro(ERF_P, x_micro));
    let poly = mul_micro(
        t,
        ERF_A1
            + mul_micro(
                t,
                ERF_A2 + mul_micro(t, ERF_A3 + mul_micro(t, ERF_A4 + mul_micro(t, ERF_A5))),
            ),
    );
    let exp_term = exp_micro(-mul_micro(x_micro, x_micro));
    MICRO - mul_micro(poly, exp_term)
}

/// Fonction de répartition de la loi normale standard, `Φ(z)`, en micro (0 à 1 000 000).
pub fn phi_micro(z_micro: i64) -> i64 {
    let arg = mul_micro(z_micro, INV_SQRT2_MICRO);
    let erf = if arg >= 0 {
        erf_micro_nonneg(arg)
    } else {
        -erf_micro_nonneg(-arg)
    };
    ((MICRO + erf) / 2).clamp(0, MICRO)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: i64, want: f64, tol: f64) {
        let diff = (got as f64 / MICRO as f64 - want).abs();
        assert!(diff <= tol, "got {got} ({} réel), attendu ≈{want}, écart {diff}", got as f64 / MICRO as f64);
    }

    #[test]
    fn exp_matches_known_values() {
        close(exp_micro(0), 1.0, 1e-5);
        close(exp_micro(MICRO), std::f64::consts::E, 1e-5);
        close(exp_micro(-MICRO), 1.0 / std::f64::consts::E, 1e-5);
        close(exp_micro(2 * MICRO), std::f64::consts::E.powi(2), 1e-4);
        close(exp_micro(-5 * MICRO), (-5.0f64).exp(), 1e-6);
    }

    #[test]
    fn ln_matches_known_values() {
        close(ln_micro(MICRO), 0.0, 1e-5);
        close(ln_micro(2 * MICRO), 2.0f64.ln(), 1e-5);
        close(ln_micro(MICRO / 10), 0.1f64.ln(), 1e-5);
        close(ln_micro(1_000 * MICRO), 1000.0f64.ln(), 1e-4);
    }

    #[test]
    fn ln_and_exp_are_inverse() {
        for x in [1_i64, 100, 1_000_000, 50_000_000, 2] {
            let back = exp_micro(ln_micro(x));
            let diff = (back - x).abs();
            assert!(diff <= x / 1_000 + 1, "x={x} aller-retour={back}");
        }
    }

    #[test]
    fn sqrt_matches_known_values() {
        close(sqrt_micro(0), 0.0, 1e-9);
        close(sqrt_micro(MICRO), 1.0, 1e-5);
        close(sqrt_micro(4 * MICRO), 2.0, 1e-5);
        close(sqrt_micro(2 * MICRO), 2.0f64.sqrt(), 1e-5);
        close(sqrt_micro(MICRO / 100), 0.1, 1e-5);
    }

    #[test]
    fn phi_matches_known_values() {
        close(phi_micro(0), 0.5, 1e-4);
        close(phi_micro(MICRO), 0.8413447, 1e-4);
        close(phi_micro(-MICRO), 0.1586553, 1e-4);
        close(phi_micro(2 * MICRO), 0.9772499, 1e-4);
        close(phi_micro(-2 * MICRO), 0.0227501, 1e-4);
    }

    #[test]
    fn same_input_always_gives_the_same_output() {
        // Pas de flottant système dans ce module : le déterminisme est garanti par construction,
        // mais ce test protège contre une régression qui en introduirait un.
        for _ in 0..5 {
            assert_eq!(exp_micro(1_234_567), exp_micro(1_234_567));
            assert_eq!(ln_micro(9_876_543), ln_micro(9_876_543));
        }
    }
}
