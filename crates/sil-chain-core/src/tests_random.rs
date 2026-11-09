//! Invariants de monnaie sous opérations aléatoires (graines fixes, reproductibles).

use crate::tests::*;
use sil_types::{Hash32, PublicKey};

/// Suite pseudo-aléatoire déterministe (xorshift).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
pub(super) fn random_operations_never_break_the_money_invariants() {
    for seed in 1..=40u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut e = escrow(BUDGET + rng.next() % 200);
        let mut now = 0u64;
        for _ in 0..300 {
            now += rng.next() % 400_000;
            match rng.next() % 5 {
                0 | 1 => {
                    let mut a = args((rng.next() % 5) as u32);
                    a.attempt = (rng.next() % 18) as u32;
                    a.result = Hash32::from_bytes([(rng.next() % 3) as u8; 32]);
                    let members: Vec<PublicKey> = (0..(rng.next() % 5))
                        .map(|_| key(10 + (rng.next() % 5) as u8))
                        .collect();
                    let _ = e.settle(&a, &members, &key(200), &eligible());
                }
                2 => {
                    let signer = if rng.next().is_multiple_of(4) {
                        key(60)
                    } else {
                        key(100)
                    };
                    let _ = e.refund(&signer, now);
                }
                3 => {
                    let _ = e.top_up(&key(100), rng.next() % 50);
                }
                _ => {}
            }
            assert!(e.paid() + e.refunded() <= e.deposited(), "graine {seed}");
            assert_eq!(e.balance(), e.deposited() - e.paid() - e.refunded());
            assert!(e.settled_count() <= 3);
            assert_eq!(
                e.paid(),
                PER_FRAGMENT * e.settled_count() as u64,
                "graine {seed}"
            );
            if e.is_closed() {
                assert_eq!(e.balance(), 0);
            }
        }
    }
}
