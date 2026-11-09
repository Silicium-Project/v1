//! Compte de réputation d'un vérificateur : PDA `[VERIFIER_SEED, clé]`, 8 octets (`i64` LE).
//!
//! Créé une fois par `RegisterVerifier` à `VERIFIER_START`. Le règlement le lit (gate : sans compte
//! ou sous le seuil, pas de paiement) et le crédite. Il n'y a pas de pénalité (ADR 0007).

use crate::process::{invalid, require_signer_writable, require_writable};
use sil_chain_core::reputation::VERIFIER_START;
use sil_chain_core::wire::VERIFIER_SEED;
use sil_types::PublicKey;
use solana_program::account_info::{AccountInfo, next_account_info};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_system_interface::{instruction as system_instruction, program as system_program};

/// Taille du compte : un score `i64` little-endian.
pub const ACCOUNT_LEN: usize = 8;

/// Adresse du compte de réputation d'une clé.
pub fn address(program_id: &Pubkey, key: &PublicKey) -> Pubkey {
    let target = Pubkey::new_from_array(*key.as_bytes());
    Pubkey::find_program_address(&[VERIFIER_SEED, target.as_ref()], program_id).0
}

/// `[payeur (signataire, inscriptible), compte (PDA, inscriptible), système]`.
///
/// Refuse une clé déjà enregistrée. Un compte pré-financé (solde non nul) est aussi refusé : même
/// contrainte que l'ouverture d'un escrow.
pub fn register(program_id: &Pubkey, accounts: &[AccountInfo], key: PublicKey) -> ProgramResult {
    let mut iter = accounts.iter();
    let payer = next_account_info(&mut iter)?;
    let account = next_account_info(&mut iter)?;
    let system = next_account_info(&mut iter)?;
    require_signer_writable(payer)?;
    require_writable(account)?;
    let target = Pubkey::new_from_array(*key.as_bytes());
    let (pda, bump) = Pubkey::find_program_address(&[VERIFIER_SEED, target.as_ref()], program_id);
    if *account.key != pda || *system.key != system_program::ID {
        return Err(invalid());
    }
    if account.lamports() > 0 || !account.data_is_empty() {
        return Err(invalid());
    }
    let rent = Rent::get()?.minimum_balance(ACCOUNT_LEN);
    let create =
        system_instruction::create_account(payer.key, &pda, rent, ACCOUNT_LEN as u64, program_id);
    invoke_signed(
        &create,
        &[payer.clone(), account.clone(), system.clone()],
        &[&[VERIFIER_SEED, target.as_ref(), &[bump]]],
    )?;
    write(account, VERIFIER_START)
}

/// Le compte de réputation de `key` parmi les comptes de l'instruction, s'il y figure.
pub fn find<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    key: &PublicKey,
) -> Option<AccountInfo<'a>> {
    let pda = address(program_id, key);
    accounts.iter().find(|account| *account.key == pda).cloned()
}

/// Score de `key` : `None` si le compte n'existe pas (jamais enregistré).
pub fn standing(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    key: &PublicKey,
) -> Result<Option<i64>, ProgramError> {
    match find(program_id, accounts, key) {
        Some(account) => read_optional(&account, program_id),
        None => Ok(None),
    }
}

/// Score d'un compte : `None` s'il est vide (pas encore créé), erreur s'il n'est pas le nôtre.
pub fn read_optional(
    account: &AccountInfo,
    program_id: &Pubkey,
) -> Result<Option<i64>, ProgramError> {
    if account.data_is_empty() && account.lamports() == 0 {
        return Ok(None);
    }
    if account.owner != program_id || account.data_len() != ACCOUNT_LEN {
        return Err(invalid());
    }
    let data = account.try_borrow_data()?;
    let bytes: [u8; ACCOUNT_LEN] = data[..ACCOUNT_LEN].try_into().map_err(|_| invalid())?;
    Ok(Some(i64::from_le_bytes(bytes)))
}

/// Écrit le score. Le compte doit être inscriptible et déjà créé.
pub fn write(account: &AccountInfo, score: i64) -> ProgramResult {
    require_writable(account)?;
    let mut data = account.try_borrow_mut_data()?;
    let target = data.get_mut(..ACCOUNT_LEN).ok_or(invalid())?;
    target.copy_from_slice(&score.to_le_bytes());
    Ok(())
}
