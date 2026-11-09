use crate::spec::SPEC_VERSION;
use crate::*;
use sil_attest::{Attestation, AttestationBody, Certificate};
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_quota::LedgerLimits;
use sil_types::{Hash32, JobId, TaskRef};

pub fn ident(n: u8) -> SigningIdentity {
    MasterSeed::from_bytes([n; 32]).derive(Purpose::Ledger)
}

pub fn hash(n: u8) -> Hash32 {
    Hash32::from_bytes([n; 32])
}

#[derive(Clone)]
pub struct Params {
    pub verifications: u32,
    pub fragments: u32,
    pub max_attempts: u32,
    pub expires_at_ms: u64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            verifications: 1,
            fragments: 1,
            max_attempts: 3,
            expires_at_ms: u64::MAX / 2,
        }
    }
}

pub fn spec_for(requester: &SigningIdentity, p: &Params) -> JobSpec {
    JobSpec {
        version: SPEC_VERSION,
        requester: requester.public(),
        nonce: 1,
        workload: "calcul/sha256".into(),
        input: hash(1),
        input_size: 100,
        width: 0,
        height: 0,
        fragments: p.fragments,
        verifications_required: p.verifications,
        lease_ms: 60_000,
        max_attempts: p.max_attempts,
        reward_per_unit: 10,
        verifier_reward_per_unit: 1,
        expires_at_ms: p.expires_at_ms,
    }
}

/// Un job publié et le journal de tous les événements émis.
pub struct World {
    pub ledger: Ledger,
    pub job: JobId,
    pub now: u64,
    pub log: Vec<Event>,
}

impl World {
    pub fn new(p: Params) -> Self {
        Self::with_limits(p, LedgerLimits::default())
    }

    pub fn with_limits(p: Params, limits: LedgerLimits) -> Self {
        let requester = ident(100);
        let mut ledger = Ledger::open_to_all(limits);
        let (job, event) = ledger
            .publish_job(&requester, spec_for(&requester, &p), 1_000)
            .unwrap();
        Self {
            ledger,
            job,
            now: 1_000,
            log: vec![event],
        }
    }

    pub fn try_emit(
        &mut self,
        who: &SigningIdentity,
        body: EventBody,
    ) -> Result<Event, LedgerError> {
        self.now += 1;
        let event = self.ledger.append_local(who, self.job, body, self.now)?;
        self.log.push(event.clone());
        Ok(event)
    }

    pub fn emit(&mut self, who: &SigningIdentity, body: EventBody) -> Event {
        self.try_emit(who, body).unwrap()
    }

    pub fn task(&self) -> TaskState {
        self.ledger.task_state(&self.job, 0, self.now).unwrap()
    }

    pub fn status(&self) -> TaskStatus {
        self.task().status
    }

    /// `worker` propose la tentative : le premier claim valide l'emporte (ADR 0005). Retourne
    /// l'id du claim.
    pub fn claim_and_accept(&mut self, worker: &SigningIdentity, attempt: u32) -> Hash32 {
        let claim = self.emit(
            worker,
            EventBody::Claim {
                fragment: 0,
                attempt,
            },
        );
        claim.id().unwrap()
    }

    pub fn submit(&mut self, worker: &SigningIdentity, attempt: u32, result: Hash32) {
        self.emit(
            worker,
            EventBody::Submit {
                fragment: 0,
                attempt,
                result,
                size: 32,
                milestones_root: hash(9),
            },
        );
    }

    /// `verifier` propose de vérifier : retenu si son offre fait partie des k premières (phase 3).
    pub fn accept_verifier(&mut self, verifier: &SigningIdentity, attempt: u32) {
        self.emit(
            verifier,
            EventBody::VerifyOffer {
                fragment: 0,
                attempt,
            },
        );
    }

    pub fn attestation(
        &self,
        verifier: &SigningIdentity,
        attempt: u32,
        result: Hash32,
        approved: bool,
    ) -> Attestation {
        Attestation::sign(
            verifier,
            AttestationBody {
                task: TaskRef::new(self.job, 0),
                attempt,
                result,
                approved,
                evidence: (!approved).then(|| hash(7)),
            },
        )
        .unwrap()
    }

    pub fn attest(
        &mut self,
        verifier: &SigningIdentity,
        attempt: u32,
        result: Hash32,
        approved: bool,
    ) {
        let attestation = self.attestation(verifier, attempt, result, approved);
        self.emit(verifier, EventBody::Attest { attestation });
    }

    pub fn certificate(
        &self,
        approvers: &[&SigningIdentity],
        attempt: u32,
        result: Hash32,
    ) -> Certificate {
        Certificate {
            task: TaskRef::new(self.job, 0),
            attempt,
            result,
            approvals: approvers
                .iter()
                .map(|v| self.attestation(v, attempt, result, true))
                .collect(),
        }
    }

    /// Claim et résultat de la tentative `attempt` : ce que désigne un `Finalize`.
    pub fn refs(&self, attempt: u32) -> (Hash32, Hash32) {
        let task = self.task();
        let state = &task.attempts[attempt as usize - 1];
        (state.claim_event, state.result.as_ref().unwrap().event)
    }

    /// Offres des vérificateurs qui approuvent le certificat (ce que désigne un `Finalize`).
    pub fn offers_of(&self, attempt: u32, certificate: &Certificate) -> Vec<Hash32> {
        let task = self.task();
        let state = &task.attempts[attempt as usize - 1];
        certificate
            .approvals
            .iter()
            .filter_map(|a| {
                state
                    .verifiers
                    .iter()
                    .find(|v| v.verifier == a.verifier)
                    .map(|v| v.offer_event)
            })
            .collect()
    }

    /// Finalise avec `copies` nœuds distincts : tout nœud peut certifier (ADR 0007).
    pub fn finalize(&mut self, copies: usize, attempt: u32, certificate: &Certificate) {
        let (claim, submit) = self.refs(attempt);
        let verifiers = self.offers_of(attempt, certificate);
        for index in 0..copies {
            let node = ident(10 + index as u8);
            self.emit(
                &node,
                EventBody::Finalize {
                    fragment: 0,
                    attempt,
                    claim,
                    submit,
                    verifiers: verifiers.clone(),
                    certificate: certificate.clone(),
                },
            );
        }
    }

    /// Déroule tout jusqu'à `Verified` avec un seul vérificateur.
    pub fn run_to_verified(
        &mut self,
        worker: &SigningIdentity,
        verifier: &SigningIdentity,
    ) -> Hash32 {
        let result = hash(42);
        self.claim_and_accept(worker, 1);
        self.submit(worker, 1, result);
        self.accept_verifier(verifier, 1);
        self.attest(verifier, 1, result, true);
        result
    }
}

/// Livre des événements à un journal, en ignorant les erreurs (retourne celles-ci).
pub fn feed(ledger: &mut Ledger, events: &[Event]) -> Vec<LedgerError> {
    events
        .iter()
        .filter_map(|event| ledger.receive(event.clone()).err())
        .collect()
}
