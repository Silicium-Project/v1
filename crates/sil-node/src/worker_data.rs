//! Données d'un worker : ce que ce nœud a fait, lu dans sa propre chaîne d'événements, avec sa
//! réputation et les règlements qu'il a vus. Lecture seule : rien n'est calculé sur la chaîne.

use crate::shared::{Shared, lock};
use serde_json::{Value, json};
use sil_ledger::EventBody;
use std::collections::BTreeMap;

/// Nombre d'événements récents renvoyés (le reste est dans les compteurs).
const RECENT: usize = 50;

/// Nom stable d'un type d'événement, tel que l'API et l'interface le montrent.
pub fn kind_of(body: &EventBody) -> &'static str {
    match body {
        EventBody::JobPublished { .. } => "job_published",
        EventBody::Claim { .. } => "claim",
        EventBody::Accept { .. } => "accept",
        EventBody::Renew { .. } => "renew",
        EventBody::Release { .. } => "release",
        EventBody::Submit { .. } => "submit",
        EventBody::VerifyOffer { .. } => "verify_offer",
        EventBody::VerifierAccept { .. } => "verifier_accept",
        EventBody::Attest { .. } => "attest",
        EventBody::Finalize { .. } => "finalize",
        EventBody::Cancel => "cancel",
    }
}

/// Rapport du nœud en tant que worker. `payments` reste faux tant que l'adaptateur de chaîne
/// n'existe pas : aucun gain n'est donc affiché comme acquis.
pub fn report(shared: &Shared) -> Value {
    let me = shared.author();
    let now = shared.now_ms();
    let mut counts: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut recent: Vec<(u64, Value)> = Vec::new();
    let mut jobs = 0u64;
    {
        let ledger = lock(&shared.ledger);
        for job in ledger.job_ids() {
            let events = ledger.events_of(&job, &me);
            if events.is_empty() {
                continue;
            }
            jobs += 1;
            for event in events {
                let kind = kind_of(&event.body);
                *counts.entry(kind).or_insert(0) += 1;
                recent.push((
                    event.issued_at_ms,
                    json!({
                        "job": job.to_hex(),
                        "sequence": event.sequence,
                        "kind": kind,
                        "issued_at_ms": event.issued_at_ms,
                    }),
                ));
            }
        }
    }
    recent.sort_by_key(|event| std::cmp::Reverse(event.0));
    recent.truncate(RECENT);

    let reputation = lock(&shared.reputation).score(&me, now);
    let mut settlements: BTreeMap<String, u64> = BTreeMap::new();
    for (_, status) in shared.settlement_report() {
        *settlements.entry(format!("{status:?}")).or_insert(0) += 1;
    }

    json!({
        "worker": me.to_hex(),
        "roles": {
            "compute": shared.roles.worker,
            "verify": shared.roles.verifier,
            "coordinate": shared.roles.coordinator,
        },
        "jobs_touched": jobs,
        "events_by_kind": counts,
        "recent": recent.into_iter().map(|(_, event)| event).collect::<Vec<_>>(),
        "reputation": reputation,
        "settlements_seen": settlements,
        "payments": false,
    })
}
