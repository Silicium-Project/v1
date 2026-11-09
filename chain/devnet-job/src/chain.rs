//! Escrow sur devnet : ouverture par le requester, attente de confirmation, lecture du règlement.

use sil_chain_core::{Escrow, EscrowTerms};
use sil_crypto::{MasterSeed, Purpose};
use sil_settlement::{Chain, TxStatus};
use sil_settlement_solana::{SolanaChain, parse_program_id};
use sil_types::JobId;
use solana_sdk::signature::Keypair;
use std::{thread::sleep, time::Duration};

/// Compte Solana du requester : la même clé ed25519 que son identité du ledger.
pub fn keypair_of(seed: &MasterSeed) -> Keypair {
    let secret = seed.derive_raw(Purpose::Ledger);
    Keypair::new_from_array(*secret.expose())
}

pub struct OnChain {
    chain: SolanaChain,
    timeout: Duration,
}

impl OnChain {
    pub fn new(
        rpc: &str,
        program: &str,
        seed: &MasterSeed,
        timeout_s: u64,
    ) -> Result<Self, String> {
        let program = parse_program_id(program).map_err(|e| format!("{e:?}"))?;
        Ok(Self {
            chain: SolanaChain::new(rpc, program, keypair_of(seed)),
            timeout: Duration::from_secs(timeout_s),
        })
    }

    /// Lecture seule : pas de graine, donc aucune signature possible.
    pub fn new_readonly(rpc: &str, program: &str, timeout_s: u64) -> Result<Self, String> {
        let program = parse_program_id(program).map_err(|e| format!("{e:?}"))?;
        let nobody = Keypair::new();
        Ok(Self {
            chain: SolanaChain::new(rpc, program, nobody),
            timeout: Duration::from_secs(timeout_s),
        })
    }

    /// Ouvre l'escrow du job avec `deposit` lamports, et attend la confirmation.
    pub fn open(
        &self,
        seed: &MasterSeed,
        job: &JobId,
        terms: EscrowTerms,
        deposit: u64,
    ) -> Result<(), String> {
        let requester = keypair_of(seed);
        let tx = self
            .chain
            .open_escrow(&requester, *job, terms, deposit)
            .map_err(|e| format!("{e:?}"))?;
        self.wait_tx(&tx)
    }

    /// Escrow du job, s'il existe déjà.
    pub fn escrow(&self, job: &JobId) -> Result<Option<Escrow>, String> {
        self.chain.escrow(job).map_err(|e| format!("{e:?}"))
    }

    fn wait_tx(&self, tx: &sil_settlement::TxId) -> Result<(), String> {
        let started = std::time::Instant::now();
        loop {
            match self.chain.tx_status(tx).map_err(|e| format!("{e:?}"))? {
                TxStatus::Confirmed => return Ok(()),
                TxStatus::Pending => {}
                other => return Err(format!("transaction refusée : {other:?}")),
            }
            if started.elapsed() > self.timeout {
                return Err("délai dépassé : confirmation de l'escrow".into());
            }
            sleep(Duration::from_secs(2));
        }
    }
}
