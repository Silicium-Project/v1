//! Adaptateur : implémente `sil_settlement::Chain` avec un client RPC bloquant.

use crate::errors::{failure_text, map_client};
use crate::tx;
use sil_chain_core::wire::decode_state;
use sil_chain_core::{Escrow, EscrowTerms};
use sil_settlement::{Authorization, Chain, ChainError, TxId, TxStatus};
use sil_types::JobId;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature, Signer};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;

/// Longueur d'en-tête du compte d'escrow (voir `chain/program/src/state.rs`).
const LEN_PREFIX: usize = 4;

/// Solde minimum du payeur avant une soumission : frais de transaction et une marge. En dessous,
/// la transaction ne partirait pas : on le signale tout de suite (`NoFunds`).
pub const FEE_RESERVE: u64 = 10_000;

/// Identifiant du programme (base58) tel qu'écrit dans la configuration.
pub fn parse_program_id(text: &str) -> Result<Pubkey, ChainError> {
    text.parse::<Pubkey>()
        .map_err(|e| ChainError::Rejected(format!("identifiant de programme invalide : {e}")))
}

/// Charge la clé de paiement (fichier JSON de clé Solana, comme `solana-keygen`).
pub fn read_payer(path: &std::path::Path) -> Result<Keypair, ChainError> {
    solana_sdk::signature::read_keypair_file(path)
        .map_err(|e| ChainError::Rejected(format!("clé de paiement illisible : {e}")))
}

pub struct SolanaChain {
    rpc: RpcClient,
    program: Pubkey,
    /// Paie les frais des règlements soumis par ce nœud.
    payer: Keypair,
    /// Dernier bloc valide de chaque transaction soumise : au-delà, elle ne peut plus atterrir.
    last_valid: Mutex<HashMap<String, u64>>,
}

impl SolanaChain {
    pub fn new(url: &str, program: Pubkey, payer: Keypair) -> Self {
        Self {
            rpc: RpcClient::new(url.to_string()),
            program,
            payer,
            last_valid: Mutex::new(HashMap::new()),
        }
    }

    pub fn rpc(&self) -> &RpcClient {
        &self.rpc
    }

    pub fn program(&self) -> Pubkey {
        self.program
    }

    /// Ouvre l'escrow d'un job et dépose `deposit` (lamports). Le requester signe et paie.
    pub fn open_escrow(
        &self,
        requester: &Keypair,
        job: JobId,
        terms: EscrowTerms,
        deposit: u64,
    ) -> Result<TxId, ChainError> {
        let ix = tx::open_ix(&self.program, &requester.pubkey(), job, terms, deposit)?;
        self.send(vec![ix], requester, &[requester])
    }

    /// Ajoute des fonds à un escrow ouvert.
    pub fn top_up(&self, requester: &Keypair, job: JobId, amount: u64) -> Result<TxId, ChainError> {
        let ix = tx::top_up_ix(&self.program, &requester.pubkey(), job, amount)?;
        self.send(vec![ix], requester, &[requester])
    }

    /// Rembourse le solde après l'échéance (le programme refuse avant).
    pub fn refund(&self, requester: &Keypair, job: JobId) -> Result<TxId, ChainError> {
        let ix = tx::refund_ix(&self.program, &requester.pubkey(), job)?;
        self.send(vec![ix], requester, &[requester])
    }

    /// Refuse d'envoyer si le payeur n'a pas de quoi payer les frais.
    fn ensure_fee_funds(&self) -> Result<(), ChainError> {
        let balance = self
            .rpc
            .get_balance_with_commitment(&self.payer.pubkey(), CommitmentConfig::confirmed())
            .map_err(map_client)?
            .value;
        if balance < FEE_RESERVE {
            return Err(ChainError::NoFunds);
        }
        Ok(())
    }

    fn send(
        &self,
        instructions: Vec<Instruction>,
        payer: &Keypair,
        signers: &[&Keypair],
    ) -> Result<TxId, ChainError> {
        let (blockhash, last_valid) = self
            .rpc
            .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
            .map_err(map_client)?;
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            signers,
            blockhash,
        );
        // The preflight simulation must see the same blockhash as the one we just fetched
        // (confirmed). The default `finalized` level rejects a fresh blockhash as unknown.
        let config = RpcSendTransactionConfig {
            preflight_commitment: Some(CommitmentLevel::Confirmed),
            ..RpcSendTransactionConfig::default()
        };
        let signature = self
            .rpc
            .send_transaction_with_config(&transaction, config)
            .map_err(map_client)?;
        let id = signature.to_string();
        self.last_valid
            .lock()
            .map_err(|_| ChainError::Transient("verrou empoisonné".into()))?
            .insert(id.clone(), last_valid);
        Ok(TxId(id))
    }
}

impl Chain for SolanaChain {
    fn escrow(&self, job: &JobId) -> Result<Option<Escrow>, ChainError> {
        let address = tx::escrow_address(&self.program, job);
        let account = self
            .rpc
            .get_account_with_commitment(&address, CommitmentConfig::confirmed())
            .map_err(map_client)?
            .value;
        let Some(account) = account else {
            return Ok(None);
        };
        if account.owner != self.program || account.data.len() < LEN_PREFIX {
            return Err(ChainError::Rejected("compte d'escrow inattendu".into()));
        }
        let len = u32::from_le_bytes([
            account.data[0],
            account.data[1],
            account.data[2],
            account.data[3],
        ]) as usize;
        let body = account
            .data
            .get(LEN_PREFIX..LEN_PREFIX + len)
            .ok_or_else(|| ChainError::Rejected("compte d'escrow tronqué".into()))?;
        decode_state(body, len)
            .map(Some)
            .map_err(|e| ChainError::Rejected(format!("état illisible : {e}")))
    }

    fn submit(&self, authorization: &Authorization) -> Result<TxId, ChainError> {
        self.ensure_fee_funds()?;
        let instructions =
            tx::settle_instructions(&self.program, &self.payer.pubkey(), authorization)?;
        self.send(instructions, &self.payer, &[&self.payer])
    }

    fn tx_status(&self, tx: &TxId) -> Result<TxStatus, ChainError> {
        let signature = Signature::from_str(&tx.0)
            .map_err(|e| ChainError::Rejected(format!("identifiant invalide : {e}")))?;
        let statuses = self
            .rpc
            .get_signature_statuses(&[signature])
            .map_err(map_client)?;
        match statuses.value.into_iter().next().flatten() {
            Some(status) => match status.err {
                Some(error) => Ok(TxStatus::Failed(failure_text(&error))),
                None if status.satisfies_commitment(CommitmentConfig::confirmed()) => {
                    Ok(TxStatus::Confirmed)
                }
                None => Ok(TxStatus::Pending),
            },
            None => {
                let height = self.rpc.get_block_height().map_err(map_client)?;
                let last = self
                    .last_valid
                    .lock()
                    .map_err(|_| ChainError::Transient("verrou empoisonné".into()))?
                    .get(&tx.0)
                    .copied();
                match last {
                    Some(last) if height > last => Ok(TxStatus::Expired),
                    _ => Ok(TxStatus::Pending),
                }
            }
        }
    }
}
