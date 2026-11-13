//! Compte d'un vérificateur : PDA `[VERIFIER_SEED, clé]`, 40 octets (`VerifierAccount` : score,
//! réserve totale, votes propres, marque de réserve en attente, heure du dernier vote).
//!
//! Créé une fois par `RegisterVerifier` à `VERIFIER_START`. Le règlement le lit (gate : sans compte
//! ou sous le seuil, pas de paiement), le crédite, et y verse la part en réserve (ADR 0010). La
//! réserve est aussi bookée par job, dans l'escrow : une pénalité ou une libération ne sort que ce
//! que ce job a retenu (correctif de revue ADR 0012).

use crate::process::{invalid, require_signer_writable, require_writable, rule};
use crate::state::{load, store};
use sil_chain_core::reserve::VerifierAccount;
use sil_chain_core::wire::VERIFIER_SEED;
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

/// Adresse du compte d'un vérificateur.
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
    write(account, &VerifierAccount::fresh())
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

/// Déplace des lamports d'un compte du programme vers un autre compte.
pub fn move_lamports(from: &AccountInfo, to: &AccountInfo, amount: u64) -> ProgramResult {
    **from.try_borrow_mut_lamports()? = from
        .lamports()
        .checked_sub(amount)
        .ok_or(invalid())?;
    **to.try_borrow_mut_lamports()? = to.lamports().checked_add(amount).ok_or(invalid())?;
    Ok(())
}

/// `[compte du vérificateur (inscriptible), clé du vérificateur (inscriptible), escrows (inscriptibles)...]`.
/// Libère la réserve que les escrows passés détiennent pour ce vérificateur, si l'échéance est
/// atteinte (ADR 0010). Seule la réserve de ces jobs sort, et elle va à la clé du vérificateur.
pub fn release(program_id: &Pubkey, accounts: &[AccountInfo], key: PublicKey) -> ProgramResult {
    let mut iter = accounts.iter();
    let account = next_account_info(&mut iter)?;
    let wallet = next_account_info(&mut iter)?;
    require_writable(account)?;
    require_writable(wallet)?;
    if *account.key != address(program_id, &key)
        || *wallet.key != Pubkey::new_from_array(*key.as_bytes())
    {
        return Err(invalid());
    }
    let mut state = read_optional(account, program_id)?.ok_or(invalid())?;
    let now = Clock::get()?.unix_timestamp;
    if state.reserved == 0 {
        return Err(rule(RuleError::ReserveNotDue("rien en réserve")));
    }
    // Chaque entrée de réserve se juge à son propre jalon : seules les entrées dues sortent.
    let mut total: u64 = 0;
    for escrow_account in iter {
        require_writable(escrow_account)?;
        let mut escrow = load(escrow_account, program_id)?;
        total = total
            .checked_add(escrow.release_held(&key, state.votes, now))
            .ok_or(invalid())?;
        store(escrow_account, &escrow)?;
    }
    if total == 0 {
        return Err(rule(RuleError::ReserveNotDue("rien d'échu sur ces escrows")));
    }
    state.reserved = state.reserved.checked_sub(total).ok_or(invalid())?;
    write(account, &state)?;
    move_lamports(account, wallet, total)
}
