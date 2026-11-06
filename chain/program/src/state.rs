//! Compte d'escrow : `[u32 LE longueur][état bincode d'Escrow][octets libres]`. La taille du compte
//! est fixée à l'ouverture d'après les conditions du job (un emplacement par fragment), donc le
//! loyer est payé une fois et connu à l'avance.

use sil_chain_core::wire::{CODE_INVALID_ACCOUNT, decode_state, encode_state};
use sil_chain_core::{Escrow, EscrowTerms};
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;

/// Octets réservés à l'en-tête (job, conditions, soldes) : largement au-dessus de la taille réelle.
const HEADER_BYTES: usize = 1536;
/// Octets réservés par fragment réglé pour `verifications_required = V` : entrée sérialisée de
/// `SettledFragment` (résultat, claimant, approbateurs).
const RECORD_BASE: usize = 160;
const RECORD_PER_APPROVER: usize = 34;

/// Taille du compte (en-tête de longueur compris) pour des conditions données.
pub fn account_space(terms: &EscrowTerms) -> Result<usize, ProgramError> {
    let record = RECORD_BASE + RECORD_PER_APPROVER * terms.verifications_required as usize;
    let body = HEADER_BYTES
        .checked_add(
            record
                .checked_mul(terms.fragments as usize)
                .ok_or(overflow())?,
        )
        .ok_or(overflow())?;
    Ok(body + 4)
}

/// Lit l'escrow stocké dans un compte appartenant au programme.
pub fn load(
    account: &AccountInfo,
    program_id: &solana_program::pubkey::Pubkey,
) -> Result<Escrow, ProgramError> {
    if account.owner != program_id {
        return Err(ProgramError::Custom(CODE_INVALID_ACCOUNT));
    }
    let data = account.try_borrow_data()?;
    let len = read_len(&data)?;
    decode_state(&data[4..4 + len], len).map_err(|_| ProgramError::InvalidAccountData)
}

/// Écrit l'escrow dans le compte. Échoue si l'encodage ne tient pas dans le compte.
pub fn store(account: &AccountInfo, escrow: &Escrow) -> Result<(), ProgramError> {
    let bytes = encode_state(escrow).map_err(|_| ProgramError::InvalidAccountData)?;
    let capacity = account
        .data_len()
        .checked_sub(4)
        .ok_or(ProgramError::AccountDataTooSmall)?;
    if bytes.len() > capacity {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let mut data = account.try_borrow_mut_data()?;
    let len = u32::try_from(bytes.len()).map_err(|_| overflow())?;
    data[0..4].copy_from_slice(&len.to_le_bytes());
    data[4..4 + bytes.len()].copy_from_slice(&bytes);
    Ok(())
}

fn read_len(data: &[u8]) -> Result<usize, ProgramError> {
    let head: [u8; 4] = data
        .get(0..4)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(ProgramError::InvalidAccountData)?;
    let len = u32::from_le_bytes(head) as usize;
    if len == 0 || 4 + len > data.len() {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(len)
}

fn overflow() -> ProgramError {
    ProgramError::Custom(sil_chain_core::wire::rule_code(
        &sil_chain_core::RuleError::Overflow,
    ))
}
