//! Dérivation de l'état : fonction pure des événements et de l'heure.
//!
//! Aucune dépendance à l'ordre d'arrivée : les votes sont comptés par membre (premier
//! vote par séquence), les ensembles sont triés par identifiant. Le coût d'une dérivation
//! est borné par les quotas d'un seul job (pas de rejeu de tout le journal).

mod attempt;
mod finalize;
mod index;
mod open;
mod task;
mod types;

use crate::{Event, EventBody, spec::JobSpec};
use index::View;
use sil_types::{Hash32, JobId, PublicKey};
use std::collections::{BTreeMap, BTreeSet};

pub use types::{
    AttemptState, Finalized, JobState, JobStatus, OpenOffer, ResultState, TaskState, TaskStatus,
    VerifierState,
};

pub(crate) use task::derive_job;

/// Pièces d'un job nécessaires aux requêtes ciblées.
pub(crate) struct JobParts<'a> {
    pub(crate) job: JobId,
    pub(crate) spec: &'a JobSpec,
    pub(crate) events: &'a BTreeMap<Hash32, Event>,
    pub(crate) equivocators: &'a BTreeSet<PublicKey>,
}

impl JobParts<'_> {
    fn view(&self) -> View<'_> {
        View::new(self.job, self.spec, self.events, self.equivocators)
    }

    pub(crate) fn task_state(&self, fragment: u32, now_ms: u64) -> TaskState {
        task::derive_task(&self.view(), fragment, now_ms)
    }

    /// Attestations d'approbation de vérificateurs retenus, pour bâtir un certificat.
    pub(crate) fn approvals(
        &self,
        fragment: u32,
        attempt: u32,
        now_ms: u64,
    ) -> Vec<sil_attest::Attestation> {
        let state = self.task_state(fragment, now_ms);
        let Some(current) = attempt
            .checked_sub(1)
            .and_then(|index| state.attempts.get(index as usize))
        else {
            return Vec::new();
        };
        let Some(result) = &current.result else {
            return Vec::new();
        };
        let approvers: BTreeSet<PublicKey> = current
            .verifiers
            .iter()
            .filter(|v| v.verdict == Some(true))
            .map(|v| v.verifier)
            .collect();
        let mut found: BTreeMap<PublicKey, (u64, sil_attest::Attestation)> = BTreeMap::new();
        for event in self.events.values() {
            let EventBody::Attest { attestation } = &event.body else {
                continue;
            };
            let body = &attestation.body;
            let relevant = body.task.fragment == fragment
                && body.attempt == attempt
                && body.result == result.result
                && body.approved
                && approvers.contains(&event.author)
                && !self.equivocators.contains(&event.author);
            let earlier = found
                .get(&event.author)
                .is_none_or(|(sequence, _)| event.sequence < *sequence);
            if relevant && earlier {
                found.insert(event.author, (event.sequence, attestation.clone()));
            }
        }
        found
            .into_values()
            .map(|(_, attestation)| attestation)
            .collect()
    }

    /// Approbations tolérance de vérificateurs retenus, pour bâtir un certificat tolérance.
    pub(crate) fn tolerance_approvals(
        &self,
        fragment: u32,
        attempt: u32,
        now_ms: u64,
    ) -> Vec<sil_attest::ToleranceAttestation> {
        let state = self.task_state(fragment, now_ms);
        let Some(current) = attempt
            .checked_sub(1)
            .and_then(|index| state.attempts.get(index as usize))
        else {
            return Vec::new();
        };
        let Some(result) = &current.result else {
            return Vec::new();
        };
        let approvers: BTreeSet<PublicKey> = current
            .verifiers
            .iter()
            .filter(|v| v.verdict == Some(true))
            .map(|v| v.verifier)
            .collect();
        let mut found: BTreeMap<PublicKey, (u64, sil_attest::ToleranceAttestation)> =
            BTreeMap::new();
        for event in self.events.values() {
            let EventBody::AttestTolerance { attestation } = &event.body else {
                continue;
            };
            let body = &attestation.body;
            let relevant = body.task.fragment == fragment
                && body.attempt == attempt
                && body.result == result.result
                && body.approved
                && approvers.contains(&event.author)
                && !self.equivocators.contains(&event.author);
            let earlier = found
                .get(&event.author)
                .is_none_or(|(sequence, _)| event.sequence < *sequence);
            if relevant && earlier {
                found.insert(event.author, (event.sequence, attestation.clone()));
            }
        }
        found
            .into_values()
            .map(|(_, attestation)| attestation)
            .take(sil_attest::MAX_APPROVALS)
            .collect()
    }
    pub(crate) fn open_claims(&self, fragment: u32, now_ms: u64) -> Vec<OpenOffer> {
        let view = self.view();
        let state = task::derive_task(&view, fragment, now_ms);
        open::open_claims(&view, &state)
    }

    pub(crate) fn open_verifications(&self, fragment: u32, now_ms: u64) -> Vec<OpenOffer> {
        let view = self.view();
        let state = task::derive_task(&view, fragment, now_ms);
        open::open_verifications(&view, &state)
    }

    pub(crate) fn verification_slots_open(&self, fragment: u32, now_ms: u64) -> u32 {
        let state = task::derive_task(&self.view(), fragment, now_ms);
        open::verification_slots_open(self.spec, &state, now_ms)
    }
}
