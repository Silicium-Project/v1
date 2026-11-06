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
    pub committee: usize,
    pub threshold: u32,
    pub verifications: u32,
    pub fragments: u32,
    pub max_attempts: u32,
    pub expires_at_ms: u64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            committee: 3,
            threshold: 2,
            verifications: 1,
            fragments: 1,
            max_attempts: 3,
            expires_at_ms: u64::MAX / 2,
        }
    }
}

pub fn spec_for(requester: &SigningIdentity, committee: &[SigningIdentity], p: &Params) -> JobSpec {
    JobSpec {
        version: 1,
        requester: requester.public(),
        nonce: 1,
        workload: "calcul/sha256".into(),
        input: hash(1),
        input_size: 100,
        width: 0,
        height: 0,
        fragments: p.fragments,
        committee: committee.iter().map(SigningIdentity::public).collect(),
        threshold: p.threshold,
        verifications_required: p.verifications,
        lease_ms: 60_000,
        max_attempts: p.max_attempts,
        reward_per_unit: 10,
        verifier_reward_per_unit: 1,
        expires_at_ms: p.expires_at_ms,
    }
}

/// Un job publié, son comité, et le journal de tous les événements émis.
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
        let committee: Vec<_> = (0..p.committee).map(|n| ident(10 + n as u8)).collect();
        let mut ledger = Ledger::open_to_all(limits);
        let (job, event) = ledger
            .publish_job(&requester, spec_for(&requester, &committee, &p), 1_000)
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

    /// Les `n` premiers membres du comité émettent le même vote.
    pub fn vote(&mut self, n: usize, body: &EventBody) {
        for index in 0..n {
            let member = ident(10 + index as u8);
            self.emit(&member, body.clone());
        }
    }

    pub fn task(&self) -> TaskState {
        self.ledger.task_state(&self.job, 0, self.now).unwrap()
    }

    pub fn status(&self) -> TaskStatus {
        self.task().status
    }

    /// `worker` propose la tentative et le comité la retient. Retourne l'id du claim.
    pub fn claim_and_accept(&mut self, worker: &SigningIdentity, attempt: u32) -> Hash32 {
        let claim = self.emit(
            worker,
            EventBody::Claim {
                fragment: 0,
                attempt,
            },
        );
        let claim_id = claim.id().unwrap();
        self.vote(
            2,
            &EventBody::Accept {
                fragment: 0,
                attempt,
                claim: claim_id,
            },
        );
        claim_id
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

    /// `verifier` propose, le comité le retient.
    pub fn accept_verifier(&mut self, verifier: &SigningIdentity, attempt: u32) {
        let offer = self.emit(
            verifier,
            EventBody::VerifyOffer {
                fragment: 0,
                attempt,
            },
        );
        self.vote(
            2,
            &EventBody::VerifierAccept {
                fragment: 0,
                attempt,
                offer: offer.id().unwrap(),
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

    pub fn finalize(&mut self, votes: usize, attempt: u32, certificate: &Certificate) {
        self.vote(
            votes,
            &EventBody::Finalize {
                fragment: 0,
                attempt,
                certificate: certificate.clone(),
            },
        );
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
