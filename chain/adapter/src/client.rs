//! Adaptateur : implémente `sil_settlement::Chain` avec un client RPC bloquant.

use crate::errors::{failure_text, map_client};
use crate::topups::TopUps;
use crate::tx;
use sil_chain_core::wire::decode_state;
use sil_chain_core::{Escrow, EscrowTerms};
use sil_settlement::{Authorization, Chain, ChainError, PenaltyClaim, TxId, TxStatus};
use sil_types::{JobId, PublicKey};
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
use std::time::Instant;

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
    /// Comptes approvisionnés en loyer depuis peu : pas de second transfert pendant la fenêtre.
    top_ups: Mutex<TopUps>,
}

impl SolanaChain {
    pub fn new(url: &str, program: Pubkey, payer: Keypair) -> Self {
        Self {
            rpc: RpcClient::new(url.to_string()),
            program,
            payer,
            last_valid: Mutex::new(HashMap::new()),
            top_ups: Mutex::new(TopUps::default()),
        }
    }

    /// Adaptateur en lecture seule : lit l'escrow, le statut et la réputation, ne paie rien. Un
    /// nœud qui règle par relais n'a pas de clé de paiement.
    pub fn read_only(url: &str, program: Pubkey) -> Self {
        Self::new(url, program, Keypair::new())
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
    /// Libère la réserve d'un vérificateur vers sa clé (ADR 0010). Permissionless : le payeur de
    /// ce client paie les frais ; les fonds ne vont qu'à la clé du vérificateur. Le programme refuse
    /// tant qu'aucune échéance n'est atteinte.
    pub fn release_reserve(&self, verifier: &PublicKey) -> Result<TxId, ChainError> {
        let ix = tx::release_reserve_ix(&self.program, &self.payer.pubkey(), verifier)?;
        self.send(vec![ix], &self.payer, &[&self.payer])
    }

    pub fn refund(&self, requester: &Keypair, job: JobId) -> Result<TxId, ChainError> {
        let ix = tx::refund_ix(&self.program, &requester.pubkey(), job)?;
        self.send(vec![ix], requester, &[requester])
    }

    /// Loyer des bénéficiaires du règlement (claimant et vérificateurs) qui n'ont pas de quoi
    /// recevoir leur part : le payeur du règlement le finance, une fois par compte et par fenêtre.
    /// Retourne les transferts et les comptes approvisionnés par ces transferts.
    fn rent_top_ups(
        &self,
        authorization: &Authorization,
    ) -> Result<(Vec<Instruction>, Vec<Pubkey>), ChainError> {
        let minimum = self
            .rpc
            .get_minimum_balance_for_rent_exemption(0)
            .map_err(map_client)?;
        let payer = self.payer.pubkey();
        let now = Instant::now();
        let mut payees = Vec::new();
        let claimant = std::iter::once(&authorization.claim.claimant);
        for key in claimant.chain(authorization.claim.approvers.iter()) {
            let address = Pubkey::new_from_array(*key.as_bytes());
            if address == payer || self.recently_topped(&address, now)? {
                continue;
            }
            let balance = self.rpc.get_balance(&address).map_err(map_client)?;
            payees.push((address, balance));
        }
        let topped = payees
            .iter()
            .filter(|(_, balance)| *balance < minimum)
            .map(|(address, _)| *address)
            .collect();
        Ok((tx::rent_top_ups(&payer, minimum, &payees), topped))
    }

    /// Enregistrement (ADR 0006) des approbateurs qui n'ont pas encore de compte de réputation :
    /// le payeur du règlement le crée, à `VERIFIER_START`, dans la même transaction. Sans cela, le
    /// règlement serait refusé (`IneligibleVerifier`).
    fn registrations(&self, authorization: &Authorization) -> Result<Vec<Instruction>, ChainError> {
        let payer = self.payer.pubkey();
        let mut out = Vec::new();
        for approver in &authorization.claim.approvers {
            let address = tx::verifier_address(&self.program, approver);
            let account = self
                .rpc
                .get_account_with_commitment(&address, CommitmentConfig::confirmed())
                .map_err(map_client)?
                .value;
            if account.is_none() {
                out.push(tx::register_verifier_ix(&self.program, &payer, approver)?);
            }
        }
        Ok(out)
    }

    /// Vrai si ce compte a déjà été approvisionné dans la fenêtre (voir `topups`).
    fn recently_topped(&self, address: &Pubkey, now: Instant) -> Result<bool, ChainError> {
        let mut top_ups = self
            .top_ups
            .lock()
            .map_err(|_| ChainError::Transient("verrou empoisonné".into()))?;
        Ok(top_ups.is_recent(address, now))
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
            .get_account_with_commitment(&address, CommitmentConfig::finalized())
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

    fn current_slot(&self) -> Result<u64, ChainError> {
        // Slot confirmé : un slot non confirmé peut être abandonné par la chaîne (ADR 0009).
        self.rpc
            .get_slot_with_commitment(CommitmentConfig::confirmed())
            .map_err(map_client)
    }

    fn penalize(&self, penalty: &PenaltyClaim) -> Result<TxId, ChainError> {
        self.ensure_fee_funds()?;
        let instructions = tx::penalize_instructions(
            &self.program,
            &self.payer.pubkey(),
            &penalty.job,
            &penalty.penalty,
            &penalty.requester,
            &penalty.approvers,
            (penalty.penalty.verifier, penalty.signature.clone()),
        )?;
        self.send(instructions, &self.payer, &[&self.payer])
    }

    fn pays_fees(&self) -> bool {
        true
    }

    fn verifier_standing(&self, verifier: &PublicKey) -> Result<Option<i64>, ChainError> {
        let address = tx::verifier_address(&self.program, verifier);
        let account = self
            .rpc
            .get_account_with_commitment(&address, CommitmentConfig::confirmed())
            .map_err(map_client)?
            .value;
        let Some(account) = account else {
            return Ok(None);
        };
        let bytes: [u8; 8] = account
            .data
            .get(..8)
            .and_then(|head| head.try_into().ok())
            .ok_or_else(|| ChainError::Rejected("compte de réputation tronqué".into()))?;
        Ok(Some(i64::from_le_bytes(bytes)))
    }

    fn submit(&self, authorization: &Authorization) -> Result<TxId, ChainError> {
        self.ensure_fee_funds()?;
        let mut instructions = self.registrations(authorization)?;
        let (top_ups, topped) = self.rent_top_ups(authorization)?;
        instructions.extend(top_ups);
        instructions.extend(tx::settle_instructions(
            &self.program,
            &self.payer.pubkey(),
            authorization,
        )?);
        // Mémorisé avant l'envoi : une erreur réseau après envoi peut cacher une transaction
        // partie. Si rien n'est parti, la fenêtre passe et le règlement se réessaie.
        if !topped.is_empty() {
            let mut top_ups = self
                .top_ups
                .lock()
                .map_err(|_| ChainError::Transient("verrou empoisonné".into()))?;
            top_ups.remember(&topped, Instant::now());
        }
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
                None if status.satisfies_commitment(CommitmentConfig::finalized()) => {
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
