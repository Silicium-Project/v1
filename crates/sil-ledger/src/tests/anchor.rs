//! Ancrage des claims sur un slot Solana (ADR 0009) : l'ordre suit le slot signé, pas l'heure
//! déclarée, et un ancrage nul est refusé.

use super::support::*;
use crate::*;

fn claim(anchor_slot: u64, attempt: u32) -> EventBody {
    EventBody::Claim {
        anchor_slot,
        fragment: 0,
        attempt,
    }
}

#[test]
fn an_earlier_anchor_wins_even_when_declared_later() {
    let (early_declared, early_anchored) = (ident(60), ident(61));
    let mut world = World::new(Params::default());
    // `early_declared` émet le premier (heure déclarée la plus petite) mais ancre plus tard.
    world.emit(&early_declared, claim(200, 1));
    world.emit(&early_anchored, claim(100, 1));
    assert_eq!(
        world.task().attempts[0].claimant,
        early_anchored.public(),
        "le slot le plus ancien gagne, pas l'heure déclarée la plus petite"
    );
}

#[test]
fn declared_time_only_breaks_ties_within_the_same_slot() {
    let (first, second) = (ident(60), ident(61));
    let mut world = World::new(Params::default());
    world.emit(&first, claim(150, 1));
    world.emit(&second, claim(150, 1));
    assert_eq!(world.task().attempts[0].claimant, first.public());
}

#[test]
fn a_zero_anchor_is_refused() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    assert!(
        world.try_emit(&worker, claim(0, 1)).is_err(),
        "un claim sans ancrage n'a pas de place dans l'ordre"
    );
}
