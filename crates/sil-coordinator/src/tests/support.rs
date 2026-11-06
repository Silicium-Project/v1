use crate::Coordinator;
use sil_attest::{Attestation, AttestationBody};
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_ledger::{EventBody, JobSpec, Ledger, TaskState};
use sil_quota::LedgerLimits;
use sil_reputation::{Reputation, ReputationConfig};
use sil_types::{Hash32, JobId, TaskRef};

pub fn ident(n: u8) -> SigningIdentity {
    MasterSeed::from_bytes([n; 32]).derive(Purpose::Ledger)
}

pub const RESULT: Hash32 = Hash32::from_bytes([42; 32]);

pub struct World {
    pub ledger: Ledger,
    pub job: JobId,
    pub now: u64,
    pub coordinators: Vec<Coordinator>,
    pub reputation: Reputation,
}

impl World {
    pub fn new() -> Self {
        Self::with_window(0)
    }

    pub fn with_window(window_ms: u64) -> Self {
        Self::with_tuning(window_ms, 0)
    }

    pub fn with_tuning(window_ms: u64, leader_wait_ms: u64) -> Self {
        let requester = ident(100);
        let committee: Vec<SigningIdentity> = (10..13).map(ident).collect();
        let spec = JobSpec {
            version: 1,
            requester: requester.public(),
            nonce: 1,
            workload: "calcul/sha256".into(),
            input: Hash32::from_bytes([1; 32]),
            input_size: 100,
            width: 0,
            height: 0,
            fragments: 1,
            committee: committee.iter().map(SigningIdentity::public).collect(),
            threshold: 2,
            verifications_required: 1,
            lease_ms: 60_000,
            max_attempts: 3,
            reward_per_unit: 10,
            verifier_reward_per_unit: 1,
            expires_at_ms: u64::MAX / 2,
        };
        let mut ledger = Ledger::open_to_all(LedgerLimits::default());
        let (job, _) = ledger.publish_job(&requester, spec, 1_000).unwrap();
        Self {
            ledger,
            job,
            now: 1_000,
            coordinators: committee
                .into_iter()
                .map(|identity| {
                    Coordinator::with_window(identity, window_ms).with_leader_wait(leader_wait_ms)
                })
                .collect(),
            reputation: Reputation::new(ReputationConfig::default(), [requester.public()]),
        }
    }

    pub fn emit(&mut self, who: &SigningIdentity, body: EventBody) -> sil_types::Hash32 {
        self.now += 1;
        let event = self
            .ledger
            .append_local(who, self.job, body, self.now)
            .unwrap();
        event.id().unwrap()
    }

    /// Chaque coordinateur décide puis émet ; retourne le nombre de votes émis.
    pub fn drive(&mut self) -> usize {
        self.drive_except(None)
    }

    /// Comme `drive`, sans faire agir le membre `skip` (membre muet ou en panne).
    pub fn drive_except(&mut self, skip: Option<usize>) -> usize {
        self.drive_members(|index| Some(index) != skip)
    }

    /// Fait agir seul le membre `only`.
    pub fn drive_only(&mut self, only: usize) -> usize {
        self.drive_members(|index| index == only)
    }

    /// Chaque membre retenu décide puis émet. Chaque vote entre au journal tout de suite : un
    /// membre qui décide après un autre voit les votes de celui-ci.
    fn drive_members(&mut self, acts: impl Fn(usize) -> bool) -> usize {
        let mut emitted = 0;
        for index in (0..self.coordinators.len()).filter(|i| acts(*i)) {
            let votes = self.coordinators[index].decide(
                &self.ledger,
                &self.reputation,
                &self.job,
                self.now,
            );
            for vote in votes {
                self.now += 1;
                let identity = ident(10 + index as u8);
                self.ledger
                    .append_local(&identity, self.job, vote, self.now)
                    .unwrap();
                emitted += 1;
            }
        }
        emitted
    }

    pub fn task(&self) -> TaskState {
        self.ledger.task_state(&self.job, 0, self.now).unwrap()
    }

    pub fn claim(&mut self, who: &SigningIdentity, attempt: u32) {
        self.emit(
            who,
            EventBody::Claim {
                fragment: 0,
                attempt,
            },
        );
    }

    pub fn submit(&mut self, who: &SigningIdentity, attempt: u32) {
        self.emit(
            who,
            EventBody::Submit {
                fragment: 0,
                attempt,
                result: RESULT,
                size: 32,
                milestones_root: Hash32::from_bytes([9; 32]),
            },
        );
    }

    pub fn offer(&mut self, who: &SigningIdentity, attempt: u32) {
        self.emit(
            who,
            EventBody::VerifyOffer {
                fragment: 0,
                attempt,
            },
        );
    }

    pub fn attest(&mut self, who: &SigningIdentity, attempt: u32, approved: bool) {
        let attestation = Attestation::sign(
            who,
            AttestationBody {
                task: TaskRef::new(self.job, 0),
                attempt,
                result: RESULT,
                approved,
                evidence: (!approved).then(|| Hash32::from_bytes([7; 32])),
            },
        )
        .unwrap();
        self.emit(who, EventBody::Attest { attestation });
    }
}
