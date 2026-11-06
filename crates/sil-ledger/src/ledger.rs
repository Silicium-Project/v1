use crate::{
    Admission, AllowAll, Event, EventBody, Header, JobSpec, JobState, LedgerError, OpenOffer,
    SignedJobSpec, TaskState,
    job::JobLedger,
    receipt::{Gap, Receipt},
};
use sil_crypto::SigningIdentity;
use sil_quota::LedgerLimits;
use sil_types::{Hash32, JobId, PublicKey};
use std::collections::BTreeMap;

/// Tous les jobs connus du nœud.
pub struct Ledger {
    limits: LedgerLimits,
    admission: Box<dyn Admission>,
    jobs: BTreeMap<JobId, JobLedger>,
    per_requester: BTreeMap<PublicKey, usize>,
}

impl Ledger {
    pub fn new(limits: LedgerLimits, admission: Box<dyn Admission>) -> Self {
        Self {
            limits,
            admission,
            jobs: BTreeMap::new(),
            per_requester: BTreeMap::new(),
        }
    }

    /// Journal sans politique d'admission (tests, réseau local).
    pub fn open_to_all(limits: LedgerLimits) -> Self {
        Self::new(limits, Box::new(AllowAll))
    }

    /// Reçoit un événement d'un pair (ordre quelconque).
    pub fn receive(&mut self, event: Event) -> Result<Receipt, LedgerError> {
        event.verify_signature()?;
        let (id, len) = event.id_and_len()?;
        if len > self.limits.max_event_bytes {
            return Err(LedgerError::EventTooLarge);
        }
        event.body.validate_shape(&event.job)?;
        if let EventBody::JobPublished { spec } = &event.body
            && !self.jobs.contains_key(&event.job)
        {
            let spec = spec.clone();
            return self.receive_new_job(event, id, len, spec);
        }
        let job = self
            .jobs
            .get_mut(&event.job)
            .ok_or(LedgerError::UnknownJob)?;
        let job_id = event.job;
        let pending = event.sequence > job.chains.next_sequence(&event.author);
        let ids = job.receive(event, id, len, &self.limits)?;
        Ok(Self::receipt(&self.jobs, &job_id, ids, pending))
    }

    fn receive_new_job(
        &mut self,
        event: Event,
        id: Hash32,
        len: usize,
        spec: SignedJobSpec,
    ) -> Result<Receipt, LedgerError> {
        let job_id = spec.verify()?;
        if job_id != event.job || event.author != spec.spec.requester {
            return Err(LedgerError::NotAuthorized("publication du job"));
        }
        if self.jobs.len() >= self.limits.max_jobs {
            return Err(LedgerError::TooManyJobs);
        }
        let requester = spec.spec.requester;
        let owned = self.per_requester.get(&requester).copied().unwrap_or(0);
        if owned >= self.limits.max_jobs_per_requester {
            return Err(LedgerError::TooManyJobsForRequester);
        }
        if !self.admission.admit(&spec, &job_id) {
            return Err(LedgerError::JobRejected);
        }
        let mut job = JobLedger::new(job_id, spec);
        let ids = job.receive(event, id, len, &self.limits)?;
        self.jobs.insert(job_id, job);
        *self.per_requester.entry(requester).or_default() += 1;
        Ok(Self::receipt(&self.jobs, &job_id, ids, false))
    }

    fn receipt(
        jobs: &BTreeMap<JobId, JobLedger>,
        job: &JobId,
        ids: Vec<Hash32>,
        pending: bool,
    ) -> Receipt {
        let accepted = jobs
            .get(job)
            .map(|ledger| {
                ids.iter()
                    .filter_map(|id| ledger.events.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default();
        Receipt {
            accepted,
            pending: pending && ids.is_empty(),
        }
    }

    /// Publie un nouveau job (événement local du requester).
    pub fn publish_job(
        &mut self,
        identity: &SigningIdentity,
        spec: JobSpec,
        now_ms: u64,
    ) -> Result<(JobId, Event), LedgerError> {
        let signed = SignedJobSpec::sign(identity, spec)?;
        let job = signed.verify()?;
        let header = Header {
            job,
            sequence: 1,
            previous: crate::NO_PREVIOUS,
            issued_at_ms: now_ms,
        };
        let event = Event::sign(identity, header, EventBody::JobPublished { spec: signed })?;
        self.receive(event.clone())?;
        Ok((job, event))
    }

    /// Signe et reçoit un événement local, à la suite de la chaîne de `identity`.
    pub fn append_local(
        &mut self,
        identity: &SigningIdentity,
        job: JobId,
        body: EventBody,
        now_ms: u64,
    ) -> Result<Event, LedgerError> {
        let ledger = self.jobs.get(&job).ok_or(LedgerError::UnknownJob)?;
        let author = identity.public();
        let header = Header {
            job,
            sequence: ledger.chains.next_sequence(&author),
            previous: ledger.chains.head_hash(&author),
            issued_at_ms: now_ms,
        };
        let event = Event::sign(identity, header, body)?;
        self.receive(event.clone())?;
        Ok(event)
    }

    pub fn job_ids(&self) -> Vec<JobId> {
        self.jobs.keys().copied().collect()
    }

    pub fn spec(&self, job: &JobId) -> Option<&SignedJobSpec> {
        self.jobs.get(job).map(|ledger| &ledger.signed_spec)
    }

    pub fn job_state(&self, job: &JobId, now_ms: u64) -> Option<JobState> {
        self.jobs.get(job).map(|ledger| ledger.state(now_ms))
    }

    pub fn task_state(&self, job: &JobId, fragment: u32, now_ms: u64) -> Option<TaskState> {
        let ledger = self.jobs.get(job)?;
        (fragment < ledger.signed_spec.spec.fragments)
            .then(|| ledger.parts().task_state(fragment, now_ms))
    }

    /// Claims à trancher pour un fragment.
    pub fn open_claims(&self, job: &JobId, fragment: u32, now_ms: u64) -> Vec<OpenOffer> {
        self.jobs
            .get(job)
            .filter(|ledger| fragment < ledger.signed_spec.spec.fragments)
            .map(|ledger| ledger.parts().open_claims(fragment, now_ms))
            .unwrap_or_default()
    }

    /// Propositions de vérification à trancher pour un fragment.
    pub fn open_verifications(&self, job: &JobId, fragment: u32, now_ms: u64) -> Vec<OpenOffer> {
        self.jobs
            .get(job)
            .filter(|ledger| fragment < ledger.signed_spec.spec.fragments)
            .map(|ledger| ledger.parts().open_verifications(fragment, now_ms))
            .unwrap_or_default()
    }

    /// Places de vérificateur restant à pourvoir pour un fragment.
    pub fn verification_slots_open(&self, job: &JobId, fragment: u32, now_ms: u64) -> u32 {
        self.jobs
            .get(job)
            .filter(|ledger| fragment < ledger.signed_spec.spec.fragments)
            .map_or(0, |ledger| {
                ledger.parts().verification_slots_open(fragment, now_ms)
            })
    }

    /// Attestations d'approbation à mettre dans un certificat de finalisation.
    pub fn approvals(
        &self,
        job: &JobId,
        fragment: u32,
        attempt: u32,
        now_ms: u64,
    ) -> Vec<sil_attest::Attestation> {
        self.jobs
            .get(job)
            .filter(|ledger| fragment < ledger.signed_spec.spec.fragments)
            .map(|ledger| ledger.parts().approvals(fragment, attempt, now_ms))
            .unwrap_or_default()
    }

    /// Votes des membres du comité pour retenir un claim (fragment, tentative).
    pub fn claim_votes(
        &self,
        job: &JobId,
        fragment: u32,
        attempt: u32,
    ) -> Vec<(PublicKey, Hash32)> {
        self.jobs
            .get(job)
            .filter(|ledger| fragment < ledger.signed_spec.spec.fragments)
            .map(|ledger| ledger.parts().claim_votes(fragment, attempt))
            .unwrap_or_default()
    }

    /// Événements d'un auteur dans un job, en ordre de séquence.
    pub fn events_of(&self, job: &JobId, author: &PublicKey) -> Vec<Event> {
        self.events_after(job, author, 0, usize::MAX)
    }

    /// Dernière séquence connue de chaque auteur du job (pour demander la suite à un pair).
    pub fn heads(&self, job: &JobId) -> BTreeMap<PublicKey, u64> {
        self.jobs
            .get(job)
            .map(|ledger| ledger.chains.heads())
            .unwrap_or_default()
    }

    /// Événements de `author` après `sequence`, en ordre de séquence, bornés (réponse à un pair).
    pub fn events_after(
        &self,
        job: &JobId,
        author: &PublicKey,
        sequence: u64,
        max: usize,
    ) -> Vec<Event> {
        let Some(ledger) = self.jobs.get(job) else {
            return Vec::new();
        };
        let mut events: Vec<&Event> = ledger
            .events
            .values()
            .filter(|e| e.author == *author && e.sequence > sequence)
            .collect();
        events.sort_by_key(|e| e.sequence);
        events.into_iter().take(max).cloned().collect()
    }

    /// Événements du job à partir d'une position de **ce** journal (ordre d'acceptation), au plus
    /// `max`. Sert à synchroniser un pair retardataire : il rappelle avec `cursor += reçus`.
    pub fn events_since(&self, job: &JobId, cursor: usize, max: usize) -> Vec<Event> {
        let Some(ledger) = self.jobs.get(job) else {
            return Vec::new();
        };
        ledger
            .order
            .iter()
            .skip(cursor)
            .take(max)
            .filter_map(|id| ledger.events.get(id).cloned())
            .collect()
    }

    /// Trous à combler : un par (job, auteur) ayant des événements en attente.
    pub fn gaps(&self) -> Vec<Gap> {
        self.jobs
            .values()
            .flat_map(|ledger| {
                ledger.pending.keys().map(|author| Gap {
                    job: ledger.job,
                    author: *author,
                    after_sequence: ledger.chains.next_sequence(author) - 1,
                })
            })
            .collect()
    }

    /// Auteurs pris à signer deux versions d'un même événement dans ce job.
    pub fn equivocators(&self, job: &JobId) -> Vec<PublicKey> {
        self.jobs
            .get(job)
            .map(|ledger| ledger.chains.equivocators.iter().copied().collect())
            .unwrap_or_default()
    }
}
