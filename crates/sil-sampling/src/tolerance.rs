//! Vérification par tolérance (`sampling_tolerance`, docs/technical/embedding-workload.md).
//!
//! Le vérificateur compare chaque vecteur reçu à son propre recalcul par cosinus. Le verdict est
//! local : aucun flottant n'entre dans un message signé (le corps signé porte un entier, à part).

/// Cosinus entre deux vecteurs. `None` si les longueurs diffèrent, si un vecteur est vide ou nul,
/// ou si une valeur n'est pas finie : un résultat mal formé ne passe jamais.
pub fn cosine(received: &[f32], recomputed: &[f32]) -> Option<f64> {
    if received.is_empty() || received.len() != recomputed.len() {
        return None;
    }
    let (mut dot, mut norm_received, mut norm_recomputed) = (0f64, 0f64, 0f64);
    for (&x, &y) in received.iter().zip(recomputed) {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let (x, y) = (f64::from(x), f64::from(y));
        dot += x * y;
        norm_received += x * x;
        norm_recomputed += y * y;
    }
    if norm_received <= 0.0 || norm_recomputed <= 0.0 {
        return None;
    }
    Some(dot / (norm_received.sqrt() * norm_recomputed.sqrt()))
}

/// Vrai si chaque couple (reçu, recalculé) atteint `cos_min`, et s'il y a au moins un couple.
/// Un seul couple sous le seuil, ou aucun couple, fait refuser le fragment.
pub fn accepts<'a, I>(pairs: I, cos_min: f64) -> bool
where
    I: IntoIterator<Item = (&'a [f32], &'a [f32])>,
{
    let mut checked = false;
    for (received, recomputed) in pairs {
        checked = true;
        match cosine(received, recomputed) {
            Some(value) if value >= cos_min => {}
            _ => return false,
        }
    }
    checked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn identical_vectors_have_cosine_one() {
        let v = [1.0f32, 2.0, 3.0];
        assert!(close(cosine(&v, &v).unwrap(), 1.0));
    }

    #[test]
    fn orthogonal_vectors_have_cosine_zero() {
        assert!(close(cosine(&[1.0, 0.0], &[0.0, 1.0]).unwrap(), 0.0));
    }

    #[test]
    fn opposite_vectors_have_cosine_minus_one() {
        assert!(close(cosine(&[1.0, 2.0], &[-1.0, -2.0]).unwrap(), -1.0));
    }

    #[test]
    fn scaling_does_not_change_the_cosine() {
        assert!(close(
            cosine(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]).unwrap(),
            1.0
        ));
    }

    #[test]
    fn malformed_vectors_have_no_cosine() {
        assert!(cosine(&[1.0, 2.0], &[1.0]).is_none(), "longueurs");
        assert!(cosine(&[], &[]).is_none(), "vide");
        assert!(cosine(&[0.0, 0.0], &[1.0, 1.0]).is_none(), "vecteur nul");
        assert!(cosine(&[f32::NAN, 1.0], &[1.0, 1.0]).is_none(), "NaN");
        assert!(
            cosine(&[f32::INFINITY, 1.0], &[1.0, 1.0]).is_none(),
            "infini"
        );
    }

    #[test]
    fn honest_float_noise_is_accepted() {
        // Écart d'ordre 1e-7 entre deux noyaux de calcul : très en dessous de 0,999.
        let recomputed = [0.12f32, -0.53, 0.8, 0.27];
        let received = [0.12f32 + 1e-8, -0.53 + 1e-8, 0.8 + 1e-7, 0.27 - 1e-8];
        assert!(accepts([(&received[..], &recomputed[..])], 0.999));
    }

    #[test]
    fn a_cheap_cheat_is_rejected_at_the_threshold() {
        // Cosinus 0,976 : l'ordre de grandeur de la quantification int8 mesurée.
        let honest = [1.0f32, 0.0];
        let cheat = [0.976f32, (1.0f32 - 0.976 * 0.976).sqrt()];
        assert!(!accepts([(&cheat[..], &honest[..])], 0.999));
        assert!(accepts([(&cheat[..], &honest[..])], 0.9));
    }

    #[test]
    fn every_pair_must_reach_the_threshold() {
        let a = [1.0f32, 0.0];
        let b = [0.0f32, 1.0];
        assert!(accepts([(&a[..], &a[..]), (&a[..], &a[..])], 0.999));
        assert!(!accepts([(&a[..], &a[..]), (&b[..], &a[..])], 0.999));
    }

    #[test]
    fn nothing_to_check_is_not_accepted() {
        assert!(!accepts(std::iter::empty::<(&[f32], &[f32])>(), 0.999));
    }
}
