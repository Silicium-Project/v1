//! Règlement d'un fragment. Chaque approbateur signe le message de règlement hors chaîne ; chaque
//! signature est vérifiée par le précompilé Ed25519 dans la même transaction. Le programme ne
//! fait que relire ces instructions : il reconstruit le message attendu et ne compte que les
//! signataires dont le précompilé porte exactement ce message.

use crate::process::{invalid, rule};
use crate::state::{load, store};
use crate::verifier;
use sil_chain_core::SettleArgs;
use sil_chain_core::wire::settlement_frame;
use sil_types::PublicKey;
use solana_program::account_info::{AccountInfo, next_account_info};
use solana_program::clock::Clock;
use solana_program::ed25519_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;
use solana_program::sysvar::instructions::{self, load_instruction_at_checked};

/// Marque « donnée dans cette instruction » du précompilé Ed25519.
const SAME_INSTRUCTION: u16 = u16::MAX;
/// Taille d'une entrée d'offsets du précompilé : sept `u16`.
const OFFSET_ENTRY: usize = 14;

/// Comptes : `[payeur (signataire, inscriptible), escrow, sysvar instructions, claimant,
/// approbateurs..., comptes de réputation des approbateurs...]`. Le payeur reçoit les frais de
/// règlement ; les autres reçoivent leur part. Chaque approbateur doit avoir un compte de
/// réputation éligible (ADR 0006) : le compte est lu, puis crédité en écriture. Il reçoit aussi sa
/// part en réserve (ADR 0010). Le compteur global des règlements (inscriptible) suit.
pub fn settle(program_id: &Pubkey, accounts: &[AccountInfo], args: SettleArgs) -> ProgramResult {
    let mut iter = accounts.iter();
    let payer = next_account_info(&mut iter)?;
    let escrow_account = next_account_info(&mut iter)?;
    let ix_sysvar = next_account_info(&mut iter)?;
    if !payer.is_signer || !payer.is_writable || !escrow_account.is_writable {
        return Err(invalid());
    }
    if *ix_sysvar.key != instructions::ID {
        return Err(invalid());
    }
    // Every account of the instruction can receive a payout: the payer (fee), the claimant and
    // the approvers. A payout to an account that was not passed is refused.
    let mut escrow = load(escrow_account, program_id)?;
    let expected = settlement_frame(&escrow.job, &args);
    let signers = verified_signers(ix_sysvar, &expected)?;
    let signers: Vec<PublicKey> = signers
        .into_iter()
        .map(|key| PublicKey::from_bytes(key.to_bytes()))
        .collect();
    let submitter = PublicKey::from_bytes(payer.key.to_bytes());

    let standing = args
        .approvers
        .iter()
        .map(|approver| verifier::standing(program_id, accounts, approver))
        .collect::<Result<Vec<_>, ProgramError>>()?;
    let settlement = escrow
        .settle(&args, &signers, &submitter, &standing)
        .map_err(rule)?;
    // Chaque approbateur compte un vote propre et garde sa part en réserve (ADR 0010).
    let now = Clock::get()?.unix_timestamp;
    for (approver, score) in &settlement.credited {
        let account = verifier::find(program_id, accounts, approver).ok_or(invalid())?;
        let mut state = verifier::read_optional(&account, program_id)?.ok_or(invalid())?;
        state.score = *score;
        let held = settlement
            .reserved
            .iter()
            .find(|(key, _)| key == approver)
            .map_or(0, |(_, amount)| *amount);
        state.record_vote(held).ok_or(invalid())?;
        // Jalon de cette réservation sur l'escrow : votes propres et heure (ADR 0010, correctif).
        escrow.stamp_booking(args.fragment, approver, state.votes, now);
        verifier::write(&account, &state)?;
        verifier::move_lamports(escrow_account, &account, held)?;
    }
    for payout in settlement.payouts {
        let key = Pubkey::new_from_array(*payout.to.as_bytes());
        let payee = accounts
            .iter()
            .find(|account| *account.key == key)
            .ok_or(invalid())?;
        if !payee.is_writable {
            return Err(invalid());
        }
        let amount = payout.amount;
        **escrow_account.try_borrow_mut_lamports()? = escrow_account
            .lamports()
            .checked_sub(amount)
            .ok_or(invalid())?;
        **payee.try_borrow_mut_lamports()? =
            payee.lamports().checked_add(amount).ok_or(invalid())?;
    }
    store(escrow_account, &escrow)
}

/// Clés dont le précompilé Ed25519 de cette transaction a vérifié **ce** message.
pub(crate) fn verified_signers(
    ix_sysvar: &AccountInfo,
    expected: &[u8],
) -> Result<Vec<Pubkey>, ProgramError> {
    let count = {
        let data = ix_sysvar.try_borrow_data()?;
        let head: [u8; 2] = data
            .get(0..2)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(invalid())?;
        u16::from_le_bytes(head)
    };
    let mut signers = Vec::new();
    for index in 0..count {
        let instruction: Instruction = load_instruction_at_checked(index as usize, ix_sysvar)?;
        if instruction.program_id != ed25519_program::ID {
            continue;
        }
        for (key, message) in parse_ed25519(&instruction.data)? {
            if message == expected && !signers.contains(&key) {
                signers.push(key);
            }
        }
    }
    Ok(signers)
}

/// Lit les entrées d'un précompilé Ed25519 qui porte ses données dans la même instruction.
/// Format : `n u8`, un octet de bourrage, puis `n` entrées de sept `u16` LE
/// (signature, pubkey, message : offset, instruction ; message : taille, instruction).
fn parse_ed25519(data: &[u8]) -> Result<Vec<(Pubkey, &[u8])>, ProgramError> {
    let count = *data.first().ok_or(invalid())? as usize;
    let mut out = Vec::with_capacity(count);
    for entry in 0..count {
        let base = 2 + OFFSET_ENTRY * entry;
        let field = |n: usize| -> Result<u16, ProgramError> {
            let at = base + 2 * n;
            let bytes: [u8; 2] = data
                .get(at..at + 2)
                .and_then(|b| b.try_into().ok())
                .ok_or(invalid())?;
            Ok(u16::from_le_bytes(bytes))
        };
        // Champs : 0 sig_off, 1 sig_ix, 2 pk_off, 3 pk_ix, 4 msg_off, 5 msg_size, 6 msg_ix.
        if field(1)? != SAME_INSTRUCTION
            || field(3)? != SAME_INSTRUCTION
            || field(6)? != SAME_INSTRUCTION
        {
            return Err(invalid());
        }
        let pk_off = field(2)? as usize;
        let msg_off = field(4)? as usize;
        let msg_size = field(5)? as usize;
        let key_bytes = data.get(pk_off..pk_off + 32).ok_or(invalid())?;
        let message = data.get(msg_off..msg_off + msg_size).ok_or(invalid())?;
        out.push((
            Pubkey::new_from_array(key_bytes.try_into().map_err(|_| invalid())?),
            message,
        ));
    }
    Ok(out)
}
