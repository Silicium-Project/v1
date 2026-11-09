//! Pièces communes du flux complet : chaîne, nœuds réels, moteur de rendu de test, job.
#![allow(dead_code)]

pub mod chain;
pub mod job;
pub mod nodes;
pub mod render;

use nodes::Running;
use sil_crypto::SigningIdentity;
use sil_settlement_solana::SolanaChain;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

/// Le monde du test : le requérant, la chaîne vue par lui, et les quatre nœuds.
pub struct World {
    pub program: Pubkey,
    pub requester: SigningIdentity,
    pub requester_key: Keypair,
    pub requester_chain: SolanaChain,
    /// Les workers, puis le vérificateur.
    pub nodes: Vec<Running>,
}
