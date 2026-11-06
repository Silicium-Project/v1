//! Les votes déjà émis par ce membre, pour ne jamais en émettre deux sur un emplacement.

use sil_ledger::{Event, EventBody};
use sil_types::Hash32;
use std::collections::BTreeSet;

pub(crate) struct Mine(pub(crate) Vec<Event>);

impl Mine {
    pub(crate) fn voted_claim(&self, fragment: u32, attempt: u32) -> bool {
        self.0.iter().any(|e| {
            matches!(&e.body, EventBody::Accept { fragment: f, attempt: a, .. }
                if *f == fragment && *a == attempt)
        })
    }

    pub(crate) fn voted_finalize(&self, fragment: u32, attempt: u32) -> bool {
        self.0.iter().any(|e| {
            matches!(&e.body, EventBody::Finalize { fragment: f, attempt: a, .. }
                if *f == fragment && *a == attempt)
        })
    }

    /// Offres pour lesquelles ce membre a déjà voté `VerifierAccept` (fragment, tentative).
    pub(crate) fn verifier_votes(&self, fragment: u32, attempt: u32) -> BTreeSet<Hash32> {
        self.0
            .iter()
            .filter_map(|e| match &e.body {
                EventBody::VerifierAccept {
                    fragment: f,
                    attempt: a,
                    offer,
                } if *f == fragment && *a == attempt => Some(*offer),
                _ => None,
            })
            .collect()
    }
}
