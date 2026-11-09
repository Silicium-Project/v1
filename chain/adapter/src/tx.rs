//! Construction des instructions : escrow (ouverture, dépôt, remboursement) et règlement avec
//! les précompilés Ed25519 des signatures des approbateurs. Les comptes et leur ordre doivent
//! correspondre exactement à `chain/program/src` (voir le commentaire de chaque fonction).

use sil_chain_core::wire::{ESCROW_SEED, EscrowIx, VERIFIER_SEED, encode_ix, settlement_frame};
use sil_chain_core::{EscrowTerms, PenaltyArgs, SettleArgs};
use sil_settlement::{Authorization, ChainError};
use sil_types::{JobId, PublicKey, Signature};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::sysvar::instructions;

/// Adresse du compte d'escrow d'un job : PDA `[ESCROW_SEED, job_id]`.
pub fn escrow_address(program: &Pubkey, job: &JobId) -> Pubkey {
    Pubkey::find_program_address(&[ESCROW_SEED, job.as_bytes()], program).0
}

fn key(public: &PublicKey) -> Pubkey {
    Pubkey::new_from_array(*public.as_bytes())
}

/// Adresse du compte de réputation d'un vérificateur : PDA `[VERIFIER_SEED, clé]` (ADR 0006).
pub fn verifier_address(program: &Pubkey, verifier: &PublicKey) -> Pubkey {
    Pubkey::find_program_address(&[VERIFIER_SEED, key(verifier).as_ref()], program).0
}

/// `[payeur (signataire, inscriptible), compte de réputation (inscriptible), système]`.
/// À envoyer une fois par vérificateur : sans ce compte, ses règlements sont refusés.
pub fn register_verifier_ix(
    program: &Pubkey,
    payer: &Pubkey,
    verifier: &PublicKey,
) -> Result<Instruction, ChainError> {
    Ok(Instruction {
        program_id: *program,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(verifier_address(program, verifier), false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
        data: data(&EscrowIx::RegisterVerifier { key: *verifier })?,
    })
}

fn data(ix: &EscrowIx) -> Result<Vec<u8>, ChainError> {
    encode_ix(ix).map_err(|e| ChainError::Rejected(format!("instruction non encodable : {e}")))
}

/// `[requester (signataire), escrow, système]`.
pub fn open_ix(
    program: &Pubkey,
    requester: &Pubkey,
    job: JobId,
    terms: EscrowTerms,
    deposit: u64,
) -> Result<Instruction, ChainError> {
    Ok(Instruction {
        program_id: *program,
        accounts: vec![
            AccountMeta::new(*requester, true),
            AccountMeta::new(escrow_address(program, &job), false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
        data: data(&EscrowIx::Open {
            job,
            terms,
            deposit,
        })?,
    })
}

/// `[requester (signataire), escrow, système]`.
pub fn top_up_ix(
    program: &Pubkey,
    requester: &Pubkey,
    job: JobId,
    amount: u64,
) -> Result<Instruction, ChainError> {
    Ok(Instruction {
        program_id: *program,
        accounts: vec![
            AccountMeta::new(*requester, true),
            AccountMeta::new(escrow_address(program, &job), false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
        data: data(&EscrowIx::TopUp { amount })?,
    })
}

/// `[requester (signataire), escrow]`.
pub fn refund_ix(
    program: &Pubkey,
    requester: &Pubkey,
    job: JobId,
) -> Result<Instruction, ChainError> {
    Ok(Instruction {
        program_id: *program,
        accounts: vec![
            AccountMeta::new(*requester, true),
            AccountMeta::new(escrow_address(program, &job), false),
        ],
        data: data(&EscrowIx::Refund)?,
    })
}

/// Un seul précompilé Ed25519 pour toutes les signatures du règlement. Le message est stocké une
/// fois : chaque entrée (signature, clé, message) pointe dessus, et le programme lit toutes les
/// entrées (`parse_ed25519`). Une instruction par signature dépasserait la taille d'une
/// transaction (1232 octets) dès trois signatures.
fn ed25519_batch_ix(message: &[u8], signatures: &[(PublicKey, Signature)]) -> Instruction {
    const SAME_INSTRUCTION: u16 = u16::MAX;
    let n = signatures.len();
    // En-tête : nombre d'entrées, bourrage, puis 14 octets par entrée. Après : 96 octets par
    // entrée (signature 64, clé 32), puis le message une seule fois.
    let base = 2 + 14 * n;
    let message_at = base + 96 * n;
    let mut header = vec![n as u8, 0];
    let mut body = Vec::with_capacity(96 * n + message.len());
    for (index, (signer, signature)) in signatures.iter().enumerate() {
        let signature_at = base + 96 * index;
        let key_at = signature_at + 64;
        for field in [
            signature_at as u16,
            SAME_INSTRUCTION,
            key_at as u16,
            SAME_INSTRUCTION,
            message_at as u16,
            message.len() as u16,
            SAME_INSTRUCTION,
        ] {
            header.extend_from_slice(&field.to_le_bytes());
        }
        body.extend_from_slice(signature.as_bytes());
        body.extend_from_slice(signer.as_bytes());
    }
    body.extend_from_slice(message);
    header.extend_from_slice(&body);
    Instruction {
        program_id: solana_sdk::ed25519_program::ID,
        accounts: Vec::new(),
        data: header,
    }
}

/// Instructions d'un règlement : un précompilé Ed25519 (les signatures), puis `settle`.
/// Comptes de `settle` : `[payeur (signataire, inscriptible), escrow, sysvar instructions,
/// claimant, approbateurs..., comptes de réputation des approbateurs...]` (tous inscriptibles :
/// les approbateurs reçoivent leur part, leur réputation est créditée).
pub fn settle_instructions(
    program: &Pubkey,
    payer: &Pubkey,
    auth: &Authorization,
) -> Result<Vec<Instruction>, ChainError> {
    let claim = &auth.claim;
    let args: SettleArgs = claim.args();
    let message = settlement_frame(&claim.job, &args);
    let mut out = vec![ed25519_batch_ix(&message, &auth.signatures)];

    let mut accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(escrow_address(program, &claim.job), false),
        AccountMeta::new_readonly(instructions::ID, false),
        AccountMeta::new(key(&claim.claimant), false),
    ];
    accounts.extend(
        claim
            .approvers
            .iter()
            .map(|approver| AccountMeta::new(key(approver), false)),
    );
    accounts.extend(
        claim
            .approvers
            .iter()
            .map(|approver| AccountMeta::new(verifier_address(program, approver), false)),
    );
    out.push(Instruction {
        program_id: *program,
        accounts,
        data: data(&EscrowIx::Settle(args))?,
    });
    Ok(out)
}

/// `[requester (signataire), escrow (inscriptible), compte de réputation (inscriptible)]`.
/// Pénalité d'un approbateur d'un fragment réglé (ADR 0008) : signée par le requester seul.
pub fn penalize_ix(
    program: &Pubkey,
    requester: &Pubkey,
    job: &JobId,
    args: &PenaltyArgs,
) -> Result<Instruction, ChainError> {
    Ok(Instruction {
        program_id: *program,
        accounts: vec![
            AccountMeta::new(*requester, true),
            AccountMeta::new(escrow_address(program, job), false),
            AccountMeta::new(verifier_address(program, &args.verifier), false),
        ],
        data: data(&EscrowIx::Penalize(args.clone()))?,
    })
}

/// Transferts du payeur vers les bénéficiaires dont le solde est sous `minimum` (loyer minimum
/// d'un compte sans données). Un compte neuf ne peut pas recevoir moins que ce minimum : le payeur
/// du règlement le finance une fois, dans la même transaction. `payees` : (adresse, solde lu).
pub fn rent_top_ups(payer: &Pubkey, minimum: u64, payees: &[(Pubkey, u64)]) -> Vec<Instruction> {
    payees
        .iter()
        .filter(|(_, balance)| *balance < minimum)
        .map(|(to, balance)| {
            solana_system_interface::instruction::transfer(payer, to, minimum - balance)
        })
        .collect()
}

#[cfg(test)]
mod verifier_tests {
    use super::*;

    #[test]
    fn the_penalty_is_signed_by_the_requester_alone() {
        let program = Pubkey::new_from_array([9; 32]);
        let requester = Pubkey::new_from_array([2; 32]);
        let args = PenaltyArgs {
            fragment: 0,
            verifier: PublicKey::from_bytes([5; 32]),
        };
        let ix = penalize_ix(&program, &requester, &JobId::from_bytes([7; 32]), &args).expect("ix");
        assert_eq!(&ix.data[..4], &5u32.to_le_bytes(), "variante Penalize");
        assert!(ix.accounts[0].is_signer);
        assert_eq!(ix.accounts.iter().filter(|a| a.is_signer).count(), 1);
    }

    #[test]
    fn register_is_the_fifth_instruction_and_targets_the_verifier_account() {
        let program = Pubkey::new_from_array([9; 32]);
        let payer = Pubkey::new_from_array([1; 32]);
        let verifier = PublicKey::from_bytes([5; 32]);
        let ix = register_verifier_ix(&program, &payer, &verifier).expect("ix");
        assert_eq!(
            &ix.data[..4],
            &4u32.to_le_bytes(),
            "variante RegisterVerifier"
        );
        assert_eq!(ix.accounts[1].pubkey, verifier_address(&program, &verifier));
        assert!(ix.accounts[1].is_writable);
        assert_ne!(
            verifier_address(&program, &verifier),
            verifier_address(&program, &PublicKey::from_bytes([6; 32])),
            "une adresse par clé"
        );
    }
}

#[cfg(test)]
mod rent_tests {
    use super::*;

    #[test]
    fn only_payees_below_the_rent_minimum_are_topped_up_by_the_difference() {
        let payer = Pubkey::new_from_array([1; 32]);
        let new_account = Pubkey::new_from_array([2; 32]);
        let funded = Pubkey::new_from_array([3; 32]);
        let ixs = rent_top_ups(&payer, 890_880, &[(new_account, 0), (funded, 2_000_000)]);
        assert_eq!(ixs.len(), 1, "un seul compte sous le minimum");
        assert_eq!(ixs[0].accounts[1].pubkey, new_account);
        let amount = u64::from_le_bytes(ixs[0].data[4..12].try_into().expect("montant"));
        assert_eq!(amount, 890_880, "le solde nul reçoit le minimum complet");
    }
}
