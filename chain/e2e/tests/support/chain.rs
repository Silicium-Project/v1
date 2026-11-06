//! Chaîne locale : configuration, comptes financés, attente de confirmation.

use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_settlement::{Chain, TxId, TxStatus};
use sil_settlement_solana::{SolanaChain, parse_program_id};
use sil_types::PublicKey;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::time::{Duration, Instant};

pub const SOL: u64 = 1_000_000_000;

/// URL RPC et identifiant du programme, donnés par `chain/scripts/localnet.sh`.
pub fn config() -> (String, Pubkey) {
    let url = std::env::var("SIL_RPC_URL").expect("SIL_RPC_URL (voir chain/README.md)");
    let program =
        parse_program_id(&std::env::var("SIL_ESCROW_PROGRAM").expect("SIL_ESCROW_PROGRAM"))
            .expect("identifiant du programme");
    (url, program)
}

pub fn ledger_identity(seed: u8) -> SigningIdentity {
    MasterSeed::from_bytes([seed; 32]).derive(Purpose::Ledger)
}

/// Compte Solana d'une identité du ledger : même clé ed25519, donc même adresse.
pub fn ledger_keypair(seed: u8) -> Keypair {
    let secret = MasterSeed::from_bytes([seed; 32]).derive_raw(Purpose::Ledger);
    Keypair::new_from_array(*secret.expose())
}

/// Adresse Solana d'une clé du ledger : c'est là que vont les paiements.
pub fn address(key: &PublicKey) -> Pubkey {
    Pubkey::new_from_array(*key.as_bytes())
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("horloge")
        .as_millis() as u64
}

/// Attend une condition, sinon fait échouer le test.
pub fn wait_for(what: &str, mut done: impl FnMut() -> bool, seconds: u64) {
    let started = Instant::now();
    while !done() {
        assert!(
            started.elapsed() < Duration::from_secs(seconds),
            "délai dépassé : {what}"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Crédite un compte (airdrop) et attend la confirmation.
pub fn airdrop(chain: &SolanaChain, to: &Pubkey, lamports: u64) {
    let signature = chain.rpc().request_airdrop(to, lamports).expect("airdrop");
    wait_for(
        "airdrop",
        || chain.rpc().confirm_transaction(&signature).unwrap_or(false),
        60,
    );
}

/// Solde confirmé d'un compte.
pub fn balance(chain: &SolanaChain, of: &Pubkey) -> u64 {
    chain
        .rpc()
        .get_balance_with_commitment(of, CommitmentConfig::confirmed())
        .expect("solde")
        .value
}

/// Attend qu'une transaction soit confirmée ; échoue si elle a été refusée ou a expiré.
pub fn confirm(chain: &dyn Chain, tx: &TxId) {
    let mut last = TxStatus::Pending;
    wait_for(
        "transaction",
        || {
            last = chain.tx_status(tx).expect("statut");
            !matches!(last, TxStatus::Pending)
        },
        120,
    );
    assert_eq!(last, TxStatus::Confirmed, "transaction non confirmée");
}
