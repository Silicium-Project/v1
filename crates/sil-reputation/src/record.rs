/// Succès et échecs pondérés d'un worker, décroissants.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Record {
    pub(crate) good: f64,
    pub(crate) bad: f64,
    pub(crate) at_ms: u64,
}

impl Record {
    pub(crate) fn new(now_ms: u64) -> Self {
        Self {
            good: 0.0,
            bad: 0.0,
            at_ms: now_ms,
        }
    }

    /// Amène les compteurs à `now_ms` : ×0,5 par demi-vie écoulée.
    pub(crate) fn decayed(self, now_ms: u64, half_life_ms: u64) -> Self {
        let elapsed = now_ms.saturating_sub(self.at_ms) as f64;
        let factor = 0.5_f64.powf(elapsed / half_life_ms.max(1) as f64);
        Self {
            good: self.good * factor,
            bad: self.bad * factor,
            at_ms: self.at_ms.max(now_ms),
        }
    }

    /// Plus rien à retenir.
    pub(crate) fn is_negligible(&self) -> bool {
        self.good + self.bad < 0.01
    }
}
