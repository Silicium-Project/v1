use super::*;
use sil_attest::{Attestation, AttestationBody, Certificate};
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_ledger::{Event, EventBody, JobSpec};
use sil_quota::LedgerLimits;
use sil_types::{Hash32, JobId, TaskRef};

const T0: u64 = 1_000;
const LEASE_MS: u64 = 60_000;
const REQUESTER: u8 = 100;

fn ident(n: u8) -> SigningIdentity {
    MasterSeed::from_bytes([n; 32]).derive(Purpose::Ledger)
}

fn spec(requester: &SigningIdentity) -> JobSpec {
    JobSpec {
        version: 2,
        requester: requester.public(),
        nonce: 1,
        workload: "calcul/sha256".into(),
        input: Hash32::from_bytes([1; 32]),
        input_size: 100,
        width: 0,
        height: 0,
        fragments: 2,
        verifications_required: 1,
        lease_ms: LEASE_MS,
        max_attempts: 3,
        reward_per_unit: 10,
        verifier_reward_per_unit: 1,
        expires_at_ms: u64::MAX / 2,
    }
}

/// Un journal écrit comme le ferait le réseau. Le comité est `ident(10..13)` ; les membres 10 et 11
/// votent ensemble, sauf quand un test dit le contraire.
struct Journal {
    ledger: Ledger,
    job: JobId,
    log: Vec<Event>,
    now: u64,
}

impl Journal {
    fn new() -> Self {
        let requester = ident(REQUESTER);
        let mut ledger = Ledger::open_to_all(LedgerLimits::default());
        let (job, event) = ledger
            .publish_job(&requester, spec(&requester), T0)
            .unwrap();
        Self {
            ledger,
            job,
            log: vec![event],
            now: T0,
        }
    }

    fn emit_at(&mut self, who: &SigningIdentity, body: EventBody, at: u64) -> Event {
        let event = self.ledger.append_local(who, self.job, body, at).unwrap();
        self.log.push(event.clone());
        event
    }

    fn emit(&mut self, who: &SigningIdentity, body: EventBody) -> Event {
        self.now += 1;
        let at = self.now;
        self.emit_at(who, body, at)
    }

    /// `worker` réclame `fragment`. Retourne l'id du claim.
    fn claim(&mut self, worker: &SigningIdentity, fragment: u32, attempt: u32) -> Hash32 {
        let claim = self.emit(worker, EventBody::Claim { fragment, attempt });
        claim.id().unwrap()
    }

    /// Une tentative sur le fragment 0, résolue jusqu'à `Finalize`, vérifiée par une personne.
    fn finish_attempt(&mut self, worker: &SigningIdentity, verifier: &SigningIdentity) {
        let result = Hash32::from_bytes([42; 32]);
        let claim = self.claim(worker, 0, 1);
        let submit = self
            .emit(
                worker,
                EventBody::Submit {
                    fragment: 0,
                    attempt: 1,
                    result,
                    size: 32,
                    milestones_root: Hash32::from_bytes([9; 32]),
                },
            )
            .id()
            .unwrap();
        let offer = self.emit(
            verifier,
            EventBody::VerifyOffer {
                fragment: 0,
                attempt: 1,
            },
        );
        let offer_id = offer.id().unwrap();
        let task = TaskRef::new(self.job, 0);
        let attest = || {
            Attestation::sign(
                verifier,
                AttestationBody {
                    task,
                    attempt: 1,
                    result,
                    approved: true,
                    evidence: None,
                },
            )
            .unwrap()
        };
        self.emit(
            verifier,
            EventBody::Attest {
                attestation: attest(),
            },
        );
        let certificate = Certificate {
            task,
            attempt: 1,
            result,
            approvals: vec![attest()],
        };
        self.emit(
            verifier,
            EventBody::Finalize {
                fragment: 0,
                attempt: 1,
                claim,
                submit,
                verifiers: vec![offer_id],
                certificate,
            },
        );
    }
}

fn trusted() -> Vec<PublicKey> {
    vec![ident(REQUESTER).public()]
}

#[test]
fn a_finalized_attempt_scores_the_worker_and_its_verifier() {
    let (worker, verifier) = (ident(50), ident(12));
    let mut journal = Journal::new();
    journal.finish_attempt(&worker, &verifier);
    let derived = derive(&journal.ledger, trusted());
    assert!(derived.evidence(&worker.public()).0 > 0.0);
    assert!(derived.evidence(&verifier.public()).0 > 0.0);
}

#[test]
fn a_replayed_journal_gives_the_same_scores() {
    let (worker, verifier) = (ident(50), ident(12));
    let mut journal = Journal::new();
    journal.finish_attempt(&worker, &verifier);
    let mut replay = Ledger::open_to_all(LedgerLimits::default());
    for event in &journal.log {
        replay.receive(event.clone()).unwrap();
    }
    let original = derive(&journal.ledger, trusted());
    let copy = derive(&replay, trusted());
    assert_eq!(original.as_of_ms(), copy.as_of_ms());
    for who in [worker.public(), verifier.public()] {
        assert_eq!(original.evidence(&who), copy.evidence(&who));
        assert_eq!(original.score(&who), copy.score(&who));
    }
}

#[test]
fn a_running_claim_counts_once_the_job_clock_passes_its_lease() {
    let (worker, other) = (ident(50), ident(51));
    let mut journal = Journal::new();
    journal.claim(&worker, 0, 1);
    let running = derive(&journal.ledger, trusted());
    assert_eq!(running.evidence(&worker.public()), (0.0, 0.0));
    // Un autre worker réclame plus tard : l'horloge du job dépasse le bail de la tentative 1.
    journal.now += 2 * LEASE_MS;
    journal.claim(&other, 1, 1);
    let expired = derive(&journal.ledger, trusted());
    assert!(expired.as_of_ms() > running.as_of_ms());
    assert!(expired.evidence(&worker.public()).1 > 0.0);
}

#[test]
fn the_cache_follows_the_journal_and_the_trusted_scopes() {
    let (worker, verifier) = (ident(50), ident(12));
    let mut journal = Journal::new();
    let mut cache = Cache::default();
    assert_eq!(
        cache
            .get(&journal.ledger, &trusted())
            .evidence(&worker.public()),
        (0.0, 0.0)
    );
    journal.finish_attempt(&worker, &verifier);
    assert!(
        cache
            .get(&journal.ledger, &trusted())
            .evidence(&worker.public())
            .0
            > 0.0
    );
    // Sans scope de confiance, le même journal ne compte rien.
    assert_eq!(
        cache.get(&journal.ledger, &[]).evidence(&worker.public()),
        (0.0, 0.0)
    );
}
