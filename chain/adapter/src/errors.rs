//! Traduction des erreurs RPC et des erreurs de transaction vers `ChainError` / `TxStatus`.
//! Règle : un doute est une erreur transitoire (on réessaiera) ; seule une règle du programme
//! ou un solde insuffisant pour les frais est définitif.

use sil_chain_core::wire::describe_code;
use sil_settlement::ChainError;
use solana_client::client_error::ClientError;
use solana_sdk::instruction::InstructionError;
use solana_sdk::transaction::TransactionError;

pub fn map_client(error: ClientError) -> ChainError {
    if let Some(tx_error) = error.get_transaction_error() {
        return map_transaction(&tx_error);
    }
    ChainError::Transient(error.to_string())
}

pub fn map_transaction(error: &TransactionError) -> ChainError {
    match error {
        TransactionError::InsufficientFundsForFee => ChainError::NoFunds,
        TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
            ChainError::Rejected(describe_code(*code).to_string())
        }
        // Blockhash expiré, compte verrouillé, etc. : nouvel essai.
        other => ChainError::Transient(other.to_string()),
    }
}

/// Texte d'une erreur de transaction atterrie (pour `TxStatus::Failed`).
pub fn failure_text(error: &TransactionError) -> String {
    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
            describe_code(*code).to_string()
        }
        other => other.to_string(),
    }
}
