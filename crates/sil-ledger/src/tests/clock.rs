use super::support::*;
use crate::EventBody;

#[test]
fn version_grows_only_with_accepted_events() {
    let mut world = World::new(Params::default());
    let before = world.ledger.version();
    assert_eq!(before, 1, "la publication du job compte");
    world.emit(
        &ident(50),
        EventBody::Claim { anchor_slot: 1,
            fragment: 0,
            attempt: 1,
        },
    );
    assert_eq!(world.ledger.version(), before + 1);
    // Rejouer un événement déjà reçu ne change pas la version.
    let again = world.log.last().cloned().unwrap();
    world.ledger.receive(again).unwrap();
    assert_eq!(world.ledger.version(), before + 1);
}

#[test]
fn head_time_is_the_issue_time_of_the_authors_last_event() {
    let mut world = World::new(Params::default());
    let worker = ident(50);
    assert_eq!(world.ledger.head_time(&world.job, &worker.public()), None);
    world.emit(
        &worker,
        EventBody::Claim { anchor_slot: 1,
            fragment: 0,
            attempt: 1,
        },
    );
    assert_eq!(
        world.ledger.head_time(&world.job, &worker.public()),
        Some(world.now)
    );
}
