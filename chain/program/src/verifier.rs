//! Compte d'un vérificateur : PDA `[VERIFIER_SEED, clé]`, 32 octets (`VerifierAccount` : score,
//! réserve, compteur et heure du dernier vote). Compteur global des règlements : PDA `[STATS_SEED]`.
//!
//! Créé une fois par `RegisterVerifier` à `VERIFIER_START`. Le règlement le lit (gate : sans compte
//! ou sous le seuil, pas de paiement), le crédite et y verse la part en réserve (ADR 0010). La
//! pénalité du requester (ADR 0008) confisque la réserve ; `ReleaseReserve` la rend à la clé quand
//! une échéance est atteinte.

use crate::process::{invalid, require_signer_writable, require_writable, rule};
use sil_chain_core::reserve::VerifierAccount;
use sil_chain_core::wire::{STATS_SEED, VERIFIER_SEED};
use sil_chain_core::RuleError;
use sil_types::PublicKey;
use solana_program::account_info::{AccountInfo, next_account_info};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_system_interface::{instruction as system_instruction, program as system_program};

/// Taille du compteur global : un `u64` little-endian.
const STATS_LEN: usize = 8;

/// Adresse du compte d'un vérificateur.
pub fn address(program_id: &Pubkey, key: &PublicKey) -> Pubkey {
    let target = Pubkey::new_from_array(*key.as_bytes());
    Pubkey::find_program_address(&[VERIFIER_SEED, target.as_ref()], program_id).0
}

/// Adresse du compteur global des règlements.
pub fn stats_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[STATS_SEED], program_id).0
}

/// `[payeur (signataire, inscriptible), compte (PDA, inscriptible), système, compteur (PDA,
/// inscriptible)]`.
///
/// Refuse une clé déjà enregistrée. Un compte pré-financé (solde non nul) est aussi refusé : même
/// contrainte que l'ouverture d'un escrow. Crée aussi le compteur global s'il n'existe pas.
pub fn register(program_id: &Pubkey, accounts: &[AccountInfo], key: PublicKey) -> ProgramResult {
    let mut iter = accounts.iter();
    let payer = next_account_info(&mut iter)?;
    let account = next_account_info(&mut iter)?;
    let system = next_account_info(&mut iter)?;
    let stats = next_account_info(&mut iter)?;
    require_signer_writable(payer)?;
    require_writable(account)?;
    require_writable(stats)?;
    let target = Pubkey::new_from_array(*key.as_bytes());
    let (pda, bump) = Pubkey::find_program_address(&[VERIFIER_SEED, target.as_ref()], program_id);
    let (stats_pda, stats_bump) = Pubkey::find_program_address(&[STATS_SEED], program_id);
    if *account.key != pda || *stats.key != stats_pda || *system.key != system_program::ID {
        return Err(invalid());
    }
    if account.lamports() > 0 || !account.data_is_empty() {
        return Err(invalid());
    }
    let rent = Rent::get()?.minimum_balance(VerifierAccount::LEN);
    let create = system_instruction::create_account(
        payer.key,
        &pda,
        rent,
        VerifierAccount::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[payer.clone(), account.clone(), system.clone()],
        &[&[VERIFIER_SEED, target.as_ref(), &[bump]]],
    )?;
    write(account, &VerifierAccount::fresh())?;

    if stats.lamports() == 0 && stats.data_is_empty() {
        let rent = Rent::get()?.minimum_balance(STATS_LEN);
        let create =
            system_instruction::create_account(payer.key, &stats_pda, rent, STATS_LEN as u64, program_id);
        invoke_signed(
            &create,
            &[payer.clone(), stats.clone(), system.clone()],
            &[&[STATS_SEED, &[stats_bump]]],
        )?;
    }
    Ok(())
}

/// Le compte de `key` parmi les comptes de l'instruction, s'il y figure.
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
        Some(account) => Ok(read_optional(&account, program_id)?.map(|state| state.score)),
        None => Ok(None),
    }
}

/// État d'un compte : `None` s'il est vide (pas encore créé), erreur s'il n'est pas le nôtre.
pub fn read_optional(
    account: &AccountInfo,
    program_id: &Pubkey,
) -> Result<Option<VerifierAccount>, ProgramError> {
    if account.data_is_empty() && account.lamports() == 0 {
        return Ok(None);
    }
    if account.owner != program_id || account.data_len() != VerifierAccount::LEN {
        return Err(invalid());
    }
    let data = account.try_borrow_data()?;
    let bytes: [u8; VerifierAccount::LEN] = data[..VerifierAccount::LEN]
        .try_into()
        .map_err(|_| invalid())?;
    Ok(Some(VerifierAccount::from_bytes(&bytes)))
}

/// Écrit l'état. Le compte doit être inscriptible et déjà créé.
pub fn write(account: &AccountInfo, state: &VerifierAccount) -> ProgramResult {
    require_writable(account)?;
    let mut data = account.try_borrow_mut_data()?;
    let target = data.get_mut(..VerifierAccount::LEN).ok_or(invalid())?;
    target.copy_from_slice(&state.to_bytes());
    Ok(())
}

/// Compteur des règlements. Le compte doit être celui du programme.
pub fn read_count(account: &AccountInfo, program_id: &Pubkey) -> Result<u64, ProgramError> {
    if *account.key != stats_address(program_id)
        || account.owner != program_id
        || account.data_len() != STATS_LEN
    {
        return Err(invalid());
    }
    let data = account.try_borrow_data()?;
    let bytes: [u8; STATS_LEN] = data[..STATS_LEN].try_into().map_err(|_| invalid())?;
    Ok(u64::from_le_bytes(bytes))
}

/// Ajoute un règlement au compteur et rend la nouvelle valeur.
pub fn bump_count(account: &AccountInfo, program_id: &Pubkey) -> Result<u64, ProgramError> {
    require_writable(account)?;
    let next = read_count(account, program_id)?
        .checked_add(1)
        .ok_or(invalid())?;
    let mut data = account.try_borrow_mut_data()?;
    data[..STATS_LEN].copy_from_slice(&next.to_le_bytes());
    Ok(next)
}

/// Déplace des lamports entre deux comptes du programme ou vers un compte quelconque : `from`
/// doit appartenir au programme.
pub fn move_lamports(from: &AccountInfo, to: &AccountInfo, amount: u64) -> ProgramResult {
    **from.try_borrow_mut_lamports()? = from
        .lamports()
        .checked_sub(amount)
        .ok_or(invalid())?;
    **to.try_borrow_mut_lamports()? = to.lamports().checked_add(amount).ok_or(invalid())?;
    Ok(())
}

/// `[compte du vérificateur (inscriptible), clé du vérificateur (inscriptible), compteur]`.
/// Verse la réserve à la clé du vérificateur si une échéance est atteinte (ADR 0010).
pub fn release(program_id: &Pubkey, accounts: &[AccountInfo], key: PublicKey) -> ProgramResult {
    let mut iter = accounts.iter();
    let account = next_account_info(&mut iter)?;
    let wallet = next_account_info(&mut iter)?;
    let stats = next_account_info(&mut iter)?;
    require_writable(account)?;
    require_writable(wallet)?;
    if *account.key != address(program_id, &key)
        || *wallet.key != Pubkey::new_from_array(*key.as_bytes())
    {
        return Err(invalid());
    }
    let mut state = read_optional(account, program_id)?.ok_or(invalid())?;
    let count = read_count(stats, program_id)?;
    let now = Clock::get()?.unix_timestamp;
    if state.reserved == 0 {
        return Err(rule(RuleError::ReserveNotDue("rien en réserve")));
    }
    if !state.is_due(count, now) {
        return Err(rule(RuleError::ReserveNotDue("échéance non atteinte")));
    }
    let amount = state.reserved;
    state.reserved = 0;
    write(account, &state)?;
    move_lamports(account, wallet, amount)
}
