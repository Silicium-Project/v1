use crate::{testing::FakeChain, *};
use sil_chain_core::{Escrow, EscrowTerms};
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_types::{Hash32, JobId, PublicKey};

pub fn ident(n: u8) -> SigningIdentity {
    MasterSeed::from_bytes([n; 32]).derive(Purpose::Ledger)
}

pub fn key(n: u8) -> PublicKey {
    PublicKey::from_bytes([n; 32])
}

pub fn job() -> JobId {
    JobId::from_bytes([1; 32])
}

pub fn terms() -> EscrowTerms {
    EscrowTerms {
        requester: ident(100).public(),
        workload: "calcul/sha256".into(),
        input_size: 20,
        width: 0,
        height: 0,
        fragments: 2,
        verifications_required: 2,
        reward_per_unit: 10,
        verifier_reward_per_unit: 1,
        expires_at_ms: 1_000_000,
    }
}

/// Escrow financé de 2 fragments (récompenses et frais).
pub fn funded_chain() -> FakeChain {
    let chain = FakeChain::new();
    let escrow = Escrow::open(
        job(),
        terms(),
        terms().max_payout().unwrap(),
        &ident(100).public(),
    )
    .unwrap();
    chain.insert_escrow(escrow);
    chain
}

pub fn claim(fragment: u32) -> SettlementClaim {
    SettlementClaim {
        job: job(),
        fragment,
        attempt: 1,
        result: Hash32::from_bytes([7; 32]),
        claimant: key(60),
        approvers: vec![ident(70).public(), ident(71).public()],
    }
}

/// Règlement signé par les approbateurs `signers` (identités 70 et 71).
pub fn authorized(claim: SettlementClaim, signers: &[u8]) -> Authorization {
    let mut auth = Authorization::new(claim.clone());
    for n in signers {
        let who = ident(*n);
        auth.add(who.public(), claim.sign(&who)).unwrap();
    }
    auth
}

pub fn outbox() -> Outbox {
    Outbox::new(OutboxConfig::default())
}

/// Avance de `step_ms` à chaque tour, `rounds` fois, en minant entre deux tours.
pub fn run(
    outbox: &mut Outbox,
    chain: &FakeChain,
    start_ms: u64,
    step_ms: u64,
    rounds: u32,
) -> u64 {
    let mut now = start_ms;
    for _ in 0..rounds {
        outbox.advance(chain, now);
        chain.mine();
        now += step_ms;
    }
    now
}
