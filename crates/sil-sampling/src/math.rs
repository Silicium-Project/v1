/// Probabilité de tomber sur au moins un des `bad` blocs falsifiés (sur `total`) en en
/// contrôlant `samples` distincts (loi hypergéométrique).
pub fn detection_probability(total: u32, bad: u32, samples: u32) -> f64 {
    if bad == 0 || samples == 0 || total == 0 {
        return 0.0;
    }
    let good = total.saturating_sub(bad);
    if samples > good {
        return 1.0;
    }
    let mut miss = 1.0_f64;
    for drawn in 0..samples {
        miss *= f64::from(good - drawn) / f64::from(total - drawn);
    }
    1.0 - miss
}

/// Plus petit échantillon qui détecte, avec une probabilité d'au moins `target`, un
/// tricheur falsifiant au moins `min_cheat_fraction` des `total` blocs (au moins un bloc).
///
/// Ordre de grandeur : pour 20 % de blocs falsifiés et 99 % de détection, une vingtaine de
/// blocs quelle que soit la taille du travail ; pour 5 %, une soixantaine.
pub fn samples_needed(total: u32, min_cheat_fraction: f64, target: f64) -> u32 {
    if total == 0 {
        return 0;
    }
    let fraction = min_cheat_fraction.clamp(0.0, 1.0);
    let bad = ((f64::from(total) * fraction).ceil() as u32).clamp(1, total);
    let target = target.clamp(0.0, 1.0);
    (1..=total)
        .find(|samples| detection_probability(total, bad, *samples) >= target)
        .unwrap_or(total)
}
