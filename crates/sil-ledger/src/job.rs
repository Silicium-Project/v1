//! Le journal d'un job : événements, chaînes, quotas, tampon d'attente, autorisations.

use crate::{
    Event, EventBody, LedgerError, SignedJobSpec,
    chain::{Chains, Class},
    state::{self, JobParts},
};
use sil_attest::CertRules;
use sil_quota::LedgerLimits;
use sil_types::{Hash32, JobId, PublicKey};
use std::collections::BTreeMap;

pub(crate) struct JobLedger {
    pub(crate) job: JobId,
    pub(crate) signed_spec: SignedJobSpec,
    pub(crate) events: BTreeMap<Hash32, Event>,
    pub(crate) chains: Chains,
    per_author: BTreeMap<PublicKey, usize>,
    bytes: usize,
    /// Identifiants dans l'ordre d'acceptation (curseur de synchronisation).
    pub(crate) order: Vec<Hash32>,
    pub(crate) pending: BTreeMap<PublicKey, BTreeMap<u64, Event>>,
}

impl JobLedger {
    pub(crate) fn new(job: JobId, signed_spec: SignedJobSpec) -> Self {
        Self {
            job,
            signed_spec,
            events: BTreeMap::new(),
            chains: Chains::default(),
            per_author: BTreeMap::new(),
            bytes: 0,
            order: Vec::new(),
            pending: BTreeMap::new(),
        }
    }

    pub(crate) fn parts(&self) -> JobParts<'_> {
        JobParts {
            job: self.job,
            spec: &self.signed_spec.spec,
            events: &self.events,
            equivocators: &self.chains.equivocators,
        }
    }

    pub(crate) fn state(&self, now_ms: u64) -> state::JobState {
        state::derive_job(
            self.job,
            &self.signed_spec.spec,
            &self.events,
            &self.chains.equivocators,
            now_ms,
        )
    }

    /// Heure d'émission du dernier événement accepté de `author` dans ce job.
    pub(crate) fn head_time(&self, author: &PublicKey) -> Option<u64> {
        let id = self.chains.head_id(author)?;
        self.events.get(&id).map(|event| event.issued_at_ms)
    }

    /// Qui peut émettre quoi. Refuse avant tout stockage pour borner le spam.
    fn authorize(&self, event: &Event, id: &Hash32) -> Result<(), LedgerError> {
        if self.events.contains_key(id) {
            return Ok(());
        }
        let spec = &self.signed_spec.spec;
        if let Some(fragment) = event.body.fragment()
            && fragment >= spec.fragments
        {
            return Err(LedgerError::InvalidEvent("fragment hors du job"));
        }
        match &event.body {
            EventBody::JobPublished { spec: published } => {
                let first = self.events.is_empty() && event.sequence == 1;
                if !first || event.author != spec.requester || *published != self.signed_spec {
                    return Err(LedgerError::NotAuthorized("publication du job"));
                }
            }
            EventBody::Cancel if event.author != spec.requester => {
                return Err(LedgerError::NotAuthorized(
                    "annulation réservée au requester",
                ));
            }
            EventBody::Attest { attestation } => {
                // Un workload en mode tolérance n'accepte que des verdicts tolérance.
                if crate::tolerance::rules_of(&spec.workload).is_some() {
                    return Err(LedgerError::InvalidEvent(
                        "attestation exacte sur un workload tolérance",
                    ));
                }
                if event.author != attestation.verifier {
                    return Err(LedgerError::NotAuthorized(
                        "l'attestation n'est pas la sienne",
                    ));
                }
                attestation.verify()?;
            }
            EventBody::Finalize { certificate, .. } => {
                if crate::tolerance::rules_of(&spec.workload).is_some() {
                    return Err(LedgerError::InvalidEvent(
                        "certificat exact sur un workload tolérance",
                    ));
                }
                certificate.verify(&CertRules {
                    required: spec.verifications_required,
                    excluded: &[],
                    eligible: &|_| true,
                })?;
            }
            EventBody::AttestTolerance { attestation } => {
                // Modèle, runtime et seuil viennent du descripteur signé, jamais du verdict.
                let (model, runtime, cos_min_micro) = crate::tolerance::rules_of(&spec.workload)
                    .ok_or(LedgerError::InvalidEvent("workload sans mode tolérance"))?;
                if event.author != attestation.verifier {
                    return Err(LedgerError::NotAuthorized(
                        "l'attestation n'est pas la sienne",
                    ));
                }
                let body = &attestation.body;
                if body.model != model || body.runtime != runtime {
                    return Err(LedgerError::InvalidEvent(
                        "modèle ou runtime hors descripteur",
                    ));
                }
                if body.approved && body.min_cos_micro < cos_min_micro {
                    return Err(LedgerError::InvalidEvent(
                        "approbation sous le seuil de cosinus",
                    ));
                }
                attestation.verify()?;
            }
            EventBody::FinalizeTolerance { certificate, .. } => {
                // Le mode vient du workload signé dans le JobSpec : jamais du certificat.
                let (model, runtime, cos_min_micro) = crate::tolerance::rules_of(&spec.workload)
                    .ok_or(LedgerError::InvalidEvent("workload sans mode tolérance"))?;
                certificate.verify(&sil_attest::ToleranceRules {
                    required: spec.verifications_required,
                    excluded: &[],
                    eligible: &|_| true,
                    model,
                    runtime,
                    cos_min_micro,
                })?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Reçoit un événement vérifié (signature, forme). Retourne les identifiants insérés.
    pub(crate) fn receive(
        &mut self,
        event: Event,
        id: Hash32,
        len: usize,
        limits: &LedgerLimits,
    ) -> Result<Vec<Hash32>, LedgerError> {
        self.authorize(&event, &id)?;
        match self.chains.classify(&event, &id, limits.max_sequence_gap)? {
            Class::Duplicate => Ok(Vec::new()),
            Class::Fork => {
                self.pending.remove(&event.author);
                self.chains.equivocators.insert(event.author);
                Err(LedgerError::Equivocation(event.author))
            }
            Class::Future => self.buffer(event, limits).map(|()| Vec::new()),
            Class::Next => {
                let author = event.author;
                self.insert(event, id, len, limits)?;
                let mut accepted = vec![id];
                self.drain(&author, limits, &mut accepted);
                Ok(accepted)
            }
        }
    }

    fn insert(
        &mut self,
        event: Event,
        id: Hash32,
        len: usize,
        limits: &LedgerLimits,
    ) -> Result<(), LedgerError> {
        if self.events.len() >= limits.max_events_per_job {
            return Err(LedgerError::JobFull);
        }
        if self.bytes.saturating_add(len) > limits.max_bytes_per_job {
            return Err(LedgerError::JobBytesFull);
        }
        let count = self.per_author.entry(event.author).or_default();
        if *count >= limits.max_events_per_author_job {
            return Err(LedgerError::AuthorQuota);
        }
        *count += 1;
        self.bytes += len;
        self.chains.record(&event, id);
        self.order.push(id);
        self.events.insert(id, event);
        Ok(())
    }

    fn buffer(&mut self, event: Event, limits: &LedgerLimits) -> Result<(), LedgerError> {
        let waiting = self.pending.entry(event.author).or_default();
        if let Some(known) = waiting.get(&event.sequence) {
            if *known == event {
                return Ok(());
            }
            let author = event.author;
            self.pending.remove(&author);
            self.chains.equivocators.insert(author);
            return Err(LedgerError::Equivocation(author));
        }
        if waiting.len() >= limits.max_pending_per_author_job {
            return Err(LedgerError::PendingFull);
        }
        waiting.insert(event.sequence, event);
        Ok(())
    }

    /// Insère les événements en attente que l'arrivée de `author` débloque.
    fn drain(&mut self, author: &PublicKey, limits: &LedgerLimits, accepted: &mut Vec<Hash32>) {
        loop {
            let next = self.chains.next_sequence(author);
            let Some(event) = self
                .pending
                .get_mut(author)
                .and_then(|waiting| waiting.remove(&next))
            else {
                break;
            };
            let Ok((id, len)) = event.id_and_len() else {
                break;
            };
            let linked = matches!(
                self.chains.classify(&event, &id, limits.max_sequence_gap),
                Ok(Class::Next)
            );
            if !linked || self.insert(event, id, len, limits).is_err() {
                // La suite ne se rattache pas (ou quota atteint) : on abandonne le tampon.
                self.pending.remove(author);
                break;
            }
            accepted.push(id);
        }
        if self.pending.get(author).is_some_and(BTreeMap::is_empty) {
            self.pending.remove(author);
        }
    }
}
