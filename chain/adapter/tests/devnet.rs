//! Cycle de vie complet de l'escrow sur devnet, avec le programme déployé (ADR 0006, 0007, 0008).
//!
//! Pas d'airdrop : les comptes sont financés depuis le payeur (`SIL_PAYER_KEYPAIR`), car le faucet
//! devnet refuse souvent. Coût : quelques centièmes de SOL.
//!
//! Lancement (WSL, depuis chain/adapter) :
//!   SIL_PAYER_KEYPAIR=/chemin/deployer.json cargo test --test devnet -- --ignored --nocapture

mod common;

use common::*;
use sil_chain_core::{PenaltyArgs, SETTLEMENT_FEE, reputation};
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_settlement::{Chain, ChainError, TxId, TxStatus};
use sil_settlement_solana::{SolanaChain, errors::map_client, read_payer, tx};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::path::PathBuf;

const DEVNET_RPC: &str = "https://api.devnet.solana.com";
const DEVNET_PROGRAM: &str = "7WzVtrXToN9wvK1TGp2RMH6R6zzodaQU6TmtpNAgNfKR";
/// Montant du requester : dépôt, loyer du compte d'escrow et frais.
const REQUESTER_FUNDS: u64 = 50_000_000;

fn devnet() -> (String, Pubkey, PathBuf) {
    let url = std::env::var("SIL_RPC_URL").unwrap_or_else(|_| DEVNET_RPC.into());
    let program = std::env::var("SIL_ESCROW_PROGRAM").unwrap_or_else(|_| DEVNET_PROGRAM.into());
    let program = parse_program(&program);
    let payer = std::env::var("SIL_PAYER_KEYPAIR").expect("SIL_PAYER_KEYPAIR (chemin du payeur)");
    (url, program, PathBuf::from(payer))
}

fn parse_program(text: &str) -> Pubkey {
    use std::str::FromStr;
    Pubkey::from_str(text).expect("identifiant de programme")
}

/// Identité d'approbateur fraîche (clé de signature = adresse Solana du compte).
fn fresh_identity() -> SigningIdentity {
    let seed: [u8; 32] = Keypair::new().to_bytes()[..32].try_into().expect("32 octets");
    MasterSeed::from_bytes(seed).derive(Purpose::Ledger)
}

/// Envoie `lamports` du payeur vers `to`, et attend la confirmation.
fn fund(chain: &SolanaChain, payer: &Keypair, to: &Pubkey, lamports: u64) {
    let ix = solana_system_interface::instruction::transfer(&payer.pubkey(), to, lamports);
    let blockhash = chain.rpc().get_latest_blockhash().expect("blockhash");
    let transaction =
        Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);
    chain
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .expect("financement");
}

/// Score de réputation lu dans le compte du vérificateur (`None` : pas de compte).
fn standing(chain: &SolanaChain, program: &Pubkey, verifier: &SigningIdentity) -> Option<i64> {
    let address = tx::verifier_address(program, &verifier.public());
    let account = chain.rpc().get_account(&address).ok()?;
    let bytes: [u8; 8] = account.data.get(..8)?.try_into().ok()?;
    Some(i64::from_le_bytes(bytes))
}

/// Pénalité signée par le requester (ADR 0008) : envoyée telle quelle, erreur du programme décodée.
fn penalize(
    chain: &SolanaChain,
    signer: &Keypair,
    job: sil_types::JobId,
    args: &PenaltyArgs,
) -> Result<TxId, ChainError> {
    let ix = tx::penalize_ix(&chain.program(), &signer.pubkey(), &job, args)?;
    let blockhash = chain
        .rpc()
        .get_latest_blockhash()
        .map_err(map_client)?;
    let transaction =
        Transaction::new_signed_with_payer(&[ix], Some(&signer.pubkey()), &[signer], blockhash);
    let signature = chain
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .map_err(map_client)?;
    Ok(TxId(signature.to_string()))
}

#[test]
#[ignore = "devnet : SIL_PAYER_KEYPAIR requis, coûte quelques centièmes de SOL"]
fn escrow_lifecycle_on_devnet() {
    let (url, program, payer_path) = devnet();
    let payer = read_payer(&payer_path).expect("lecture du payeur");
    let secret: [u8; 32] = payer.to_bytes()[..32].try_into().expect("32 octets");
    let payer_copy = Keypair::new_from_array(secret);
    let chain = SolanaChain::new(&url, program, payer_copy);

    // Acteurs : requester financé, approbateur et hors-liste sans fonds (le règlement finance leur
    // loyer), claimant sans fonds (idem).
    let requester = Keypair::new();
    fund(&chain, &payer, &requester.pubkey(), REQUESTER_FUNDS);
    let claimant = Keypair::new();
    let approver = fresh_identity();
    let outsider = fresh_identity();
    let rent_minimum = chain
        .rpc()
        .get_minimum_balance_for_rent_exemption(0)
        .expect("loyer minimal");

    // 1. Ouverture : l'escrow existe avec le dépôt voulu.
    let job = random_job();
    let expires = now_ms() + 3_600_000;
    let deposit = terms(&requester, expires).max_payout().expect("budget");
    let tx = chain
        .open_escrow(&requester, job, terms(&requester, expires), deposit)
        .expect("ouverture acceptée");
    assert_eq!(settle_status(&chain, &tx), TxStatus::Confirmed, "ouverture");
    let escrow = chain.escrow(&job).expect("lecture").expect("escrow présent");
    assert_eq!(escrow.deposited(), deposit);

    // 2. Règlement du fragment 0 signé par l'approbateur : le gate passe (pas de compte, donc
    //    enregistrement automatique à 5 000), le claimant est payé, le score monte à 5 100.
    assert_eq!(standing(&chain, &program, &approver), None, "pas encore enregistré");
    let auth = signed(claim(job, 0, &claimant, &approver), &[&approver]);
    let tx = chain.submit(&auth).expect("règlement accepté");
    assert_eq!(settle_status(&chain, &tx), TxStatus::Confirmed, "règlement fragment 0");
    let escrow = chain.escrow(&job).expect("lecture").expect("escrow présent");
    assert!(escrow.settled(0).is_some(), "fragment 0 réglé on-chain");
    assert_eq!(escrow.paid(), 1_100 + SETTLEMENT_FEE);
    let claimant_balance = confirmed_balance(&chain, &claimant.pubkey());
    assert!(claimant_balance >= rent_minimum + 1_000, "le claimant reçoit 1 000 lamports");
    assert_eq!(
        standing(&chain, &program, &approver),
        Some(reputation::VERIFIER_START + reputation::VERIFIED_STEP),
        "score crédité après règlement"
    );

    // 3. Même règlement une seconde fois : refusé.
    match chain.submit(&auth) {
        Err(ChainError::Rejected(text)) => assert!(text.contains("déjà réglé"), "{text}"),
        other => panic!("second règlement refusé attendu, reçu {other:?}"),
    }

    // 4. Fragment 1 signé par un non-approbateur : refusé (autorisation insuffisante).
    let weak = signed(claim(job, 1, &claimant, &approver), &[&outsider]);
    match chain.submit(&weak) {
        Err(ChainError::Rejected(text)) => assert!(text.contains("autorisation insuffisante"), "{text}"),
        other => panic!("signature d'un non-approbateur refusée attendue, reçu {other:?}"),
    }

    // 5. Ajout de fonds.
    let tx = chain.top_up(&requester, job, 500).expect("dépôt accepté");
    assert_eq!(settle_status(&chain, &tx), TxStatus::Confirmed, "dépôt");
    let escrow = chain.escrow(&job).expect("lecture").expect("escrow présent");
    assert_eq!(escrow.deposited(), deposit + 500);

    // 6. Remboursement avant échéance : refusé.
    match chain.refund(&requester, job) {
        Err(ChainError::Rejected(text)) => assert!(text.contains("trop tôt"), "{text}"),
        other => panic!("remboursement anticipé refusé attendu, reçu {other:?}"),
    }

    // 7. Pénalité par le requester (ADR 0008) : le score baisse de 1 000, une seule fois.
    let penalty = PenaltyArgs {
        fragment: 0,
        verifier: approver.public(),
    };
    penalize(&chain, &requester, job, &penalty).expect("pénalité acceptée");
    assert_eq!(
        standing(&chain, &program, &approver),
        Some(reputation::VERIFIER_START + reputation::VERIFIED_STEP - reputation::PENALTY_STEP),
        "score après pénalité"
    );
    match penalize(&chain, &requester, job, &penalty) {
        Err(ChainError::Rejected(text)) => assert_eq!(text, "pénalité refusée", "{text}"),
        other => panic!("seconde pénalité refusée attendue, reçu {other:?}"),
    }

    // 8. Pénalité par un autre que le requester : refusée (NotRequester).
    match penalize(&chain, &payer, job, &penalty) {
        Err(ChainError::Rejected(text)) => {
            assert_eq!(text, "seul le requester peut faire cette opération", "{text}")
        }
        other => panic!("pénalité d'un tiers refusée attendue, reçu {other:?}"),
    }

    // Le remboursement après échéance et délai de grâce n'est pas testé ici : il faudrait attendre
    // le délai de grâce du programme (24 h).
}
