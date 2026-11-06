//! Construction des instructions : escrow (ouverture, dépôt, remboursement) et règlement avec
//! les précompilés Ed25519 des signatures du comité. Les comptes et leur ordre doivent
//! correspondre exactement à `chain/program/src` (voir le commentaire de chaque fonction).

use sil_chain_core::wire::{ESCROW_SEED, EscrowIx, encode_ix, settlement_frame};
use sil_chain_core::{EscrowTerms, SettleArgs};
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
/// claimant, approbateurs...]` (tous inscriptibles : ils reçoivent leur part).
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
    out.push(Instruction {
        program_id: *program,
        accounts,
        data: data(&EscrowIx::Settle(args))?,
    });
    Ok(out)
}
