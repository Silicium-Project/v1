//! Annulation par le requester : l'événement `Cancel` est signé en texte (le wallet l'affiche),
//! son corps se code comme le texte `"Cancel"`, et il part de la tête de la chaîne du requester.

use super::support::*;
use crate::*;

/// Le requester émet son `Cancel` à partir de la tête de sa chaîne : le journal l'accepte.
#[test]
fn a_requester_cancel_from_the_chain_head_is_accepted() {
    let requester = ident(100);
    let mut world = World::new(Params::default());
    let (sequence, previous) = world
        .ledger
        .head(&world.job, &requester.public())
        .expect("la publication du job est la tête de la chaîne du requester");
    let cancel = Event::sign(
        &requester,
        Header {
            job: world.job,
            sequence: sequence + 1,
            previous,
            issued_at_ms: 2_000,
        },
        EventBody::Cancel,
    )
    .unwrap();
    assert!(cancel.signing_text().unwrap().ends_with("Body: Cancel"));
    cancel.verify_signature().unwrap();
    world.ledger.receive(cancel).unwrap();
    assert_eq!(world.task().status, TaskStatus::Cancelled);
}

/// Parité avec le web (`apps/web/src/signing/cancel.ts`) : le corps `Cancel` est le texte CBOR
/// `"Cancel"`, soit 0x66 suivi des six octets de « Cancel ».
#[test]
fn the_cancel_body_is_the_cbor_text_cancel() {
    let bytes = sil_codec::encode(&EventBody::Cancel).unwrap();
    assert_eq!(bytes, [0x66, b'C', b'a', b'n', b'c', b'e', b'l']);
}
