use crate::Authorization;
use serde::{Deserialize, Serialize};
use sil_chain_core::Escrow;
use sil_types::{JobId, PublicKey};
use thiserror::Error;

/// Identifiant d'une transaction sur la chaîne (signature Solana encodée).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxId(pub String);

/// Où en est une transaction soumise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxStatus {
    /// Pas encore confirmée, et elle peut encore l'être.
    Pending,
    Confirmed,
    /// Atterrie mais en échec (règle du programme).
    Failed(String),
    /// Ne peut plus jamais atterrir (blockhash expiré) : on peut renvoyer **après** avoir relu
    /// l'état de l'escrow.
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChainError {
    /// RPC limité (429), injoignable, délai : on ne sait rien, on réessaiera.
    #[error("chaîne indisponible : {0}")]
    Transient(String),
    /// Le payeur des frais n'a pas de SOL : ni refus définitif, ni réessai aveugle.
    #[error("fonds insuffisants pour les frais de transaction")]
    NoFunds,
    /// Le programme refuse (règle violée) : définitif pour cette transaction.
    #[error("refusé par le programme : {0}")]
    Rejected(String),
}

/// Ce que le règlement exige de la chaîne. L'adaptateur Solana implémente ce trait ;
/// `testing::FakeChain` en donne une version simulée avec pannes.
///
/// Méthodes bloquantes : le nœud les appelle depuis un thread dédié.
pub trait Chain: Send + Sync {
    /// État de l'escrow du job (`None` s'il n'existe pas).
    fn escrow(&self, job: &JobId) -> Result<Option<Escrow>, ChainError>;

    /// Soumet le règlement (avec les signatures des approbateurs). `Ok` = acceptée par le réseau,
    /// **pas** confirmée.
    fn submit(&self, authorization: &Authorization) -> Result<TxId, ChainError>;

    fn tx_status(&self, tx: &TxId) -> Result<TxStatus, ChainError>;

    /// Envoie une pénalité (ADR 0012) : preuve = verdict signé du vérificateur. `Ok` = acceptée par
    /// le réseau, pas confirmée.
    fn penalize(&self, penalty: &crate::PenaltyClaim) -> Result<TxId, ChainError>;

    /// Réputation réseau d'un vérificateur, lue on-chain (ADR 0006). `None` : compte absent (pas
    /// encore enregistré) ou chaîne non configurée.
    fn verifier_standing(&self, _verifier: &PublicKey) -> Result<Option<i64>, ChainError> {
        Ok(None)
    }

    /// Slot Solana courant, lu à la confirmation (ADR 0009). Sert d'ancrage aux claims et de borne
    /// à leur réception. `Err` si la chaîne n'est pas configurée : un nœud sans chaîne ne réclame ni
    /// ne valide de claim.
    fn current_slot(&self) -> Result<u64, ChainError> {
        Err(ChainError::Transient("slot indisponible".into()))
    }

    /// `true` si ce nœud paie lui-même les frais de transaction. Seul un tel nœud peut relayer
    /// les règlements d'autres nœuds (voir `POST /v1/settlements`).
    fn pays_fees(&self) -> bool {
        false
    }
}
