use crate::{Event, EventBody, spec::JobSpec};
use sil_types::{Hash32, JobId, PublicKey};
use std::collections::{BTreeMap, BTreeSet};

/// Événement et son identifiant.
pub(super) type Ev<'a> = (&'a Hash32, &'a Event);

/// Événements d'un job regroupés par fragment, équivocateurs écartés.
pub(super) struct View<'a> {
    pub(super) job: JobId,
    pub(super) spec: &'a JobSpec,
    by_fragment: BTreeMap<u32, Vec<Ev<'a>>>,
    pub(super) cancelled: bool,
}

impl<'a> View<'a> {
    pub(super) fn new(
        job: JobId,
        spec: &'a JobSpec,
        events: &'a BTreeMap<Hash32, Event>,
        equivocators: &BTreeSet<PublicKey>,
    ) -> Self {
        let mut by_fragment: BTreeMap<u32, Vec<Ev<'a>>> = BTreeMap::new();
        let mut cancelled = false;
        for (id, event) in events {
            if equivocators.contains(&event.author) {
                continue;
            }
            if matches!(event.body, EventBody::Cancel) {
                cancelled |= event.author == spec.requester;
            } else if let Some(fragment) = event.body.fragment() {
                by_fragment.entry(fragment).or_default().push((id, event));
            }
        }
        Self {
            job,
            spec,
            by_fragment,
            cancelled,
        }
    }

    pub(super) fn task_events(&self, fragment: u32) -> &[Ev<'a>] {
        match self.by_fragment.get(&fragment) {
            Some(events) => events,
            None => &[],
        }
    }
}

/// Ordre total des événements : heure d'émission, puis identifiant.
pub(super) fn by_time(events: &mut [Ev<'_>]) {
    events.sort_by_key(|(id, event)| (event.issued_at_ms, **id));
}
