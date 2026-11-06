use crate::{KeyedLimiter, LedgerLimits, TokenBucket};

#[test]
fn bucket_empties_then_refills_with_time() {
    let mut bucket = TokenBucket::new(3, 1, 0);
    assert!(bucket.try_take(3, 0));
    assert!(!bucket.try_take(1, 0));
    assert!(!bucket.try_take(1, 999));
    assert!(bucket.try_take(1, 1000));
}

#[test]
fn bucket_never_exceeds_capacity_and_ignores_clock_going_back() {
    let mut bucket = TokenBucket::new(2, 100, 5000);
    assert!(bucket.try_take(2, 100_000));
    assert!(!bucket.try_take(3, 100_000));
    assert!(!bucket.try_take(1, 0), "horloge en arrière : aucun gain");
}

#[test]
fn keyed_limiter_is_independent_per_key() {
    let mut limiter = KeyedLimiter::new(8, 1, 0);
    assert!(limiter.allow(&"a", 1, 0));
    assert!(!limiter.allow(&"a", 1, 0));
    assert!(limiter.allow(&"b", 1, 0));
}

#[test]
fn many_new_keys_cannot_evict_exhausted_ones() {
    let mut limiter = KeyedLimiter::new(2, 1, 0);
    assert!(limiter.allow(&1, 1, 0));
    assert!(limiter.allow(&2, 1, 0));
    // table pleine de seaux épuisés : une nouvelle clé est refusée
    assert!(!limiter.allow(&3, 1, 0));
    // le seau épuisé de la clé 1 n'a pas été remis à zéro
    assert!(!limiter.allow(&1, 1, 0));
    assert_eq!(limiter.len(), 2);
}

#[test]
fn idle_keys_are_purged_to_make_room() {
    let mut limiter = KeyedLimiter::new(1, 1, 1);
    assert!(limiter.allow(&1, 1, 0));
    assert!(limiter.allow(&2, 1, 10_000), "seau 1 revenu plein, purgé");
}

#[test]
fn default_limits_are_positive() {
    let limits = LedgerLimits::default();
    assert!(limits.max_jobs > 0 && limits.max_events_per_author_job > 0);
    assert!(limits.max_events_per_author_job <= limits.max_events_per_job);
}
