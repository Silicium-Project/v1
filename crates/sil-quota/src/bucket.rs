/// Seau à jetons en arithmétique entière (millième de jeton), horloge passée en argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenBucket {
    capacity_milli: u64,
    refill_milli_per_sec: u64,
    tokens_milli: u64,
    last_ms: u64,
}

impl TokenBucket {
    /// Seau plein : `capacity` jetons, `refill_per_sec` jetons rendus par seconde.
    pub fn new(capacity: u32, refill_per_sec: u32, now_ms: u64) -> Self {
        let capacity_milli = u64::from(capacity) * 1000;
        Self {
            capacity_milli,
            refill_milli_per_sec: u64::from(refill_per_sec) * 1000,
            tokens_milli: capacity_milli,
            last_ms: now_ms,
        }
    }

    fn refill(&mut self, now_ms: u64) {
        let elapsed = now_ms.saturating_sub(self.last_ms);
        let gained = elapsed.saturating_mul(self.refill_milli_per_sec) / 1000;
        self.tokens_milli = self
            .tokens_milli
            .saturating_add(gained)
            .min(self.capacity_milli);
        self.last_ms = self.last_ms.max(now_ms);
    }

    /// Consomme `cost` jetons si possible.
    pub fn try_take(&mut self, cost: u32, now_ms: u64) -> bool {
        self.refill(now_ms);
        let needed = u64::from(cost) * 1000;
        if self.tokens_milli >= needed {
            self.tokens_milli -= needed;
            true
        } else {
            false
        }
    }

    /// Vrai si le seau est plein à `now_ms` (équivalent à un seau neuf).
    pub fn is_full(&mut self, now_ms: u64) -> bool {
        self.refill(now_ms);
        self.tokens_milli == self.capacity_milli
    }
}
