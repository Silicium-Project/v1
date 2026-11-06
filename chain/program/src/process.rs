//! Dispatch des instructions et les opérations qui ne touchent pas aux signatures du comité.

use crate::settle;
use crate::state::{account_space, load, store};
use sil_chain_core::wire::{CODE_INVALID_ACCOUNT, ESCROW_SEED, EscrowIx, decode_ix, rule_code};
use sil_chain_core::{Escrow, EscrowTerms, RuleError};
use sil_types::{JobId, PublicKey};
use solana_program::account_info::{AccountInfo, next_account_info};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_system_interface::{instruction as system_instruction, program as system_program};

/// Plus grand message d'instruction accepté (les conditions d'un job tiennent largement).
const MAX_IX_BYTES: usize = 16 * 1024;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let ix: EscrowIx =
        decode_ix(data, MAX_IX_BYTES).map_err(|_| ProgramError::InvalidInstructionData)?;
    match ix {
        EscrowIx::Open {
            job,
            terms,
            deposit,
        } => open(program_id, accounts, job, terms, deposit),
        EscrowIx::TopUp { amount } => top_up(program_id, accounts, amount),
        EscrowIx::Settle(args) => settle::settle(program_id, accounts, args),
        EscrowIx::Refund => refund(program_id, accounts),
    }
}

/// Ouvre l'escrow : compte `[requester (signataire), escrow (PDA), système]`.
fn open(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    job: JobId,
    terms: EscrowTerms,
    deposit: u64,
) -> ProgramResult {
    let mut iter = accounts.iter();
    let requester = next_account_info(&mut iter)?;
    let escrow_account = next_account_info(&mut iter)?;
    let system = next_account_info(&mut iter)?;
    require_signer_writable(requester)?;
    require_writable(escrow_account)?;

    let (pda, bump) = Pubkey::find_program_address(&[ESCROW_SEED, job.as_bytes()], program_id);
    if *escrow_account.key != pda || *system.key != system_program::ID {
        return Err(invalid());
    }
    if escrow_account.lamports() > 0 || !escrow_account.data_is_empty() {
        return Err(invalid());
    }
    // Les règles vérifient aussi que le signataire est le requester déclaré dans les conditions.
    let requester_key = PublicKey::from_bytes(requester.key.to_bytes());
    let escrow = Escrow::open(job, terms.clone(), deposit, &requester_key).map_err(rule)?;

    let space = account_space(&terms)?;
    let rent = Rent::get()?.minimum_balance(space);
    let lamports = rent.checked_add(deposit).ok_or(invalid())?;
    let create =
        system_instruction::create_account(requester.key, &pda, lamports, space as u64, program_id);
    invoke_signed(
        &create,
        &[requester.clone(), escrow_account.clone(), system.clone()],
        &[&[ESCROW_SEED, job.as_bytes(), &[bump]]],
    )?;
    store(escrow_account, &escrow)
}

/// Ajoute des fonds : compte `[requester (signataire), escrow, système]`.
fn top_up(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let mut iter = accounts.iter();
    let requester = next_account_info(&mut iter)?;
    let escrow_account = next_account_info(&mut iter)?;
    let system = next_account_info(&mut iter)?;
    require_signer_writable(requester)?;
    require_writable(escrow_account)?;
    let mut escrow = load(escrow_account, program_id)?;
    escrow
        .top_up(&PublicKey::from_bytes(requester.key.to_bytes()), amount)
        .map_err(rule)?;
    invoke_signed(
        &system_instruction::transfer(requester.key, escrow_account.key, amount),
        &[requester.clone(), escrow_account.clone(), system.clone()],
        &[],
    )?;
    store(escrow_account, &escrow)
}

/// Rembourse le solde après l'échéance : compte `[requester (signataire), escrow]`.
///
/// Le compte n'est pas effacé : il garde l'état `closed`, donc le même job ne peut pas être
/// rouvert (sinon des fragments déjà réglés pourraient l'être à nouveau). Seul le solde revient
/// au requester ; le loyer reste bloqué dans le compte.
fn refund(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let mut iter = accounts.iter();
    let requester = next_account_info(&mut iter)?;
    let escrow_account = next_account_info(&mut iter)?;
    require_signer_writable(requester)?;
    require_writable(escrow_account)?;
    let mut escrow = load(escrow_account, program_id)?;
    let now_ms = now_ms()?;
    let amount = escrow
        .refund(&PublicKey::from_bytes(requester.key.to_bytes()), now_ms)
        .map_err(rule)?;
    store(escrow_account, &escrow)?;
    **escrow_account.try_borrow_mut_lamports()? = escrow_account
        .lamports()
        .checked_sub(amount)
        .ok_or(invalid())?;
    **requester.try_borrow_mut_lamports()? =
        requester.lamports().checked_add(amount).ok_or(invalid())?;
    Ok(())
}

fn now_ms() -> Result<u64, ProgramError> {
    let seconds = Clock::get()?.unix_timestamp;
    u64::try_from(seconds)
        .ok()
        .and_then(|s| s.checked_mul(1000))
        .ok_or(invalid())
}

fn require_signer_writable(account: &AccountInfo) -> ProgramResult {
    if !account.is_signer || !account.is_writable {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

fn require_writable(account: &AccountInfo) -> ProgramResult {
    if !account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

pub fn rule(error: RuleError) -> ProgramError {
    ProgramError::Custom(rule_code(&error))
}

pub fn invalid() -> ProgramError {
    ProgramError::Custom(CODE_INVALID_ACCOUNT)
}
