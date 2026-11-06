use std::time::Instant;

/// Horloge monotone en millisecondes, pour les limiteurs (jamais l'heure murale).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Clock {
    start: Instant,
}

impl Clock {
    pub(crate) fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub(crate) fn now_ms(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}
