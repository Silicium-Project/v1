//! Pénalité du requester (ADR 0008) : baisse le score d'un approbateur d'un fragment réglé.
//!
//! Comptes : `[requester (signataire de la transaction), escrow (inscriptible), compte de réputation
//! du vérificateur (inscriptible)]`. Aucune signature du comité : le requester signe la transaction.

use crate::process::{invalid, require_writable, rule};
use crate::state::{load, store};
use crate::verifier;
use sil_chain_core::PenaltyArgs;
use sil_types::PublicKey;
use solana_program::account_info::{AccountInfo, next_account_info};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn penalize(program_id: &Pubkey, accounts: &[AccountInfo], args: PenaltyArgs) -> ProgramResult {
    let mut iter = accounts.iter();
    let requester = next_account_info(&mut iter)?;
    let escrow_account = next_account_info(&mut iter)?;
    let standing_account = next_account_info(&mut iter)?;
    if !requester.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    require_writable(escrow_account)?;
    require_writable(standing_account)?;
    if *standing_account.key != verifier::address(program_id, &args.verifier) {
        return Err(invalid());
    }

    let mut escrow = load(escrow_account, program_id)?;
    let signer = PublicKey::from_bytes(requester.key.to_bytes());
    let current = verifier::read_optional(standing_account, program_id)?;
    let score = escrow.penalize(&args, &signer, current).map_err(rule)?;
    verifier::write(standing_account, score)?;
    store(escrow_account, &escrow)
}
