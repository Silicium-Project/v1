//! Helpers shared by the localnet scenarios (see chain/adapter/README.md).
#![allow(dead_code)]

use sil_chain_core::EscrowTerms;
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_settlement::{Authorization, Chain, SettlementClaim, TxId, TxStatus};
use sil_settlement_solana::SolanaChain;
use sil_types::{Hash32, JobId, PublicKey};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const SOL: u64 = 1_000_000_000;

pub fn config() -> (String, Pubkey) {
    let url = std::env::var("SIL_RPC_URL").expect("SIL_RPC_URL");
    let program =
        Pubkey::from_str(&std::env::var("SIL_ESCROW_PROGRAM").expect("SIL_ESCROW_PROGRAM"))
            .expect("program id");
    (url, program)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis() as u64
}

pub fn sil_key(seed: u8) -> SigningIdentity {
    MasterSeed::from_bytes([seed; 32]).derive(Purpose::Ledger)
}

/// Une clé Solana financée (les payés doivent exister avec du solde, sinon le loyer bloque).
pub fn funded(chain: &SolanaChain, lamports: u64) -> Keypair {
    let key = Keypair::new();
    let signature = chain
        .rpc()
        .request_airdrop(&key.pubkey(), lamports)
        .expect("airdrop");
    wait_for(
        || chain.rpc().confirm_transaction(&signature).unwrap_or(false),
        "airdrop",
    );
    key
}

/// Compte Solana de l'approbateur : la clé de sa signature (les paiements vont à cette clé).
pub fn approver_account(approver: &SigningIdentity) -> Pubkey {
    Pubkey::new_from_array(*approver.public().as_bytes())
}

/// Identité d'approbateur financée : un compte Solana à l'adresse de sa clé de signature.
pub fn funded_identity(chain: &SolanaChain, lamports: u64) -> SigningIdentity {
    let seed: [u8; 32] = Keypair::new().to_bytes()[..32]
        .try_into()
        .expect("32 octets");
    let identity = MasterSeed::from_bytes(seed).derive(Purpose::Ledger);
    let signature = chain
        .rpc()
        .request_airdrop(&approver_account(&identity), lamports)
        .expect("airdrop");
    wait_for(
        || chain.rpc().confirm_transaction(&signature).unwrap_or(false),
        "airdrop",
    );
    identity
}

pub fn wait_for(mut done: impl FnMut() -> bool, what: &str) {
    for _ in 0..120 {
        if done() {
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    panic!("timeout: {what}");
}

/// Attend qu'une transaction soit confirmée et retourne son statut final.
pub fn settle_status(chain: &SolanaChain, tx: &TxId) -> TxStatus {
    let mut last = TxStatus::Pending;
    wait_for(
        || {
            last = chain.tx_status(tx).expect("status");
            !matches!(last, TxStatus::Pending)
        },
        "transaction",
    );
    last
}

pub fn terms(requester: &Keypair, expires_at_ms: u64) -> EscrowTerms {
    EscrowTerms {
        requester: PublicKey::from_bytes(requester.pubkey().to_bytes()),
        // 20 octets sur 2 fragments : 10 unités chacun. Claimant : 10 × 100 = 1 000 ;
        // vérificateur : 10 × 10 = 100.
        workload: "calcul/sha256".into(),
        input_size: 20,
        width: 0,
        height: 0,
        fragments: 2,
        verifications_required: 1,
        reward_per_unit: 100,
        verifier_reward_per_unit: 10,
        expires_at_ms,
    }
}

pub fn claim(
    job: JobId,
    fragment: u32,
    claimant: &Keypair,
    approver: &SigningIdentity,
) -> SettlementClaim {
    SettlementClaim {
        job,
        fragment,
        attempt: 1,
        result: Hash32::from_bytes([7; 32]),
        claimant: PublicKey::from_bytes(claimant.pubkey().to_bytes()),
        approvers: vec![approver.public()],
    }
}

pub fn signed(claim: SettlementClaim, signers: &[&SigningIdentity]) -> Authorization {
    let mut auth = Authorization::new(claim.clone());
    for signer in signers {
        auth.add(signer.public(), claim.sign(signer))
            .expect("signature verifies");
    }
    auth
}

/// Solde lu au niveau `confirmed` (le client RPC par défaut lit `finalized`, en retard).
pub fn confirmed_balance(chain: &SolanaChain, key: &Pubkey) -> u64 {
    chain
        .rpc()
        .get_balance_with_commitment(key, CommitmentConfig::confirmed())
        .expect("balance")
        .value
}

pub fn random_job() -> JobId {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&now_ms().to_le_bytes());
    bytes[8..16].copy_from_slice(&std::process::id().to_le_bytes().repeat(2));
    JobId::from_bytes(bytes)
}
