//! Pénalité d'un vérificateur qui a voté contre la majorité (ADR 0012).
//!
//! La preuve est le verdict signé du vérificateur (`Domain::Attestation`, le même que pour un vote
//! normal), vérifié par le précompilé Ed25519 de la même transaction. Le requester ne décide rien : n'importe qui peut envoyer la
//! transaction, elle n'aboutit que si la preuve est valide.
//!
//! Comptes : `[payeur (signataire, inscriptible), escrow (inscriptible), sysvar instructions,
//! compte du vérificateur (PDA, inscriptible), portefeuille du requester (inscriptible),
//! portefeuilles des approbateurs (inscriptibles)...]`.

use crate::process::{invalid, require_signer_writable, require_writable, rule};
use crate::settle::verified_signers;
use crate::state::{load, store};
use crate::verifier;
use sil_chain_core::PenaltyArgs;
use sil_chain_core::wire::penalty_frame;
use sil_types::PublicKey;
use solana_program::account_info::{AccountInfo, next_account_info};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::instructions;

pub fn penalize(program_id: &Pubkey, accounts: &[AccountInfo], args: PenaltyArgs) -> ProgramResult {
    let mut iter = accounts.iter();
    let payer = next_account_info(&mut iter)?;
    let escrow_account = next_account_info(&mut iter)?;
    let ix_sysvar = next_account_info(&mut iter)?;
    let standing_account = next_account_info(&mut iter)?;
    let requester_wallet = next_account_info(&mut iter)?;
    require_signer_writable(payer)?;
    require_writable(escrow_account)?;
    require_writable(standing_account)?;
    require_writable(requester_wallet)?;
    if *ix_sysvar.key != instructions::ID {
        return Err(invalid());
    }
    if *standing_account.key != verifier::address(program_id, &args.verifier) {
        return Err(invalid());
    }

    let mut escrow = load(escrow_account, program_id)?;
    let expected = penalty_frame(&escrow.job, &args);
    let signed: Vec<PublicKey> = verified_signers(ix_sysvar, &expected)?
        .into_iter()
        .map(|key| PublicKey::from_bytes(key.to_bytes()))
        .collect();

    let current = verifier::read_optional(standing_account, program_id)?;
    let reserve = current.map_or(0, |state| state.reserved);
    let penalty = escrow
        .penalize(&args, &signed, current.map(|state| state.score), reserve)
        .map_err(rule)?;

    // Score et réserve du vérificateur mis à jour, puis la part confisquée est versée.
    let mut state = current.ok_or(invalid())?;
    state.score = penalty.score;
    state.reserved -= penalty.confiscated;
    verifier::write(standing_account, &state)?;
    for payout in &penalty.payouts {
        if payout.amount == 0 {
            continue;
        }
        let key = Pubkey::new_from_array(*payout.to.as_bytes());
        let payee = accounts
            .iter()
            .find(|account| *account.key == key)
            .ok_or(invalid())?;
        require_writable(payee)?;
        verifier::move_lamports(standing_account, payee, payout.amount)?;
    }
    store(escrow_account, &escrow)
}
