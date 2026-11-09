//! Vecteur figé de l'instruction `Open` : le site (TypeScript) doit produire les mêmes octets.
//! Régénération : `SIL_REGENERATE_VECTORS=1 cargo test -p sil-chain-core open_bytes`.

use crate::EscrowTerms;
use crate::wire::{EscrowIx, encode_ix};
use sil_types::{JobId, PublicKey};

const VECTOR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/vectors/escrow-open.hex"
);

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn open_bytes() -> String {
    let terms = EscrowTerms {
        requester: PublicKey::from_bytes([9; 32]),
        workload: "calcul/sha256".into(),
        input_size: 2048,
        width: 0,
        height: 0,
        fragments: 2,
        verifications_required: 1,
        reward_per_unit: 100,
        verifier_reward_per_unit: 10,
        expires_at_ms: 1_800_000_000_000,
    };
    let ix = EscrowIx::Open {
        job: JobId::from_bytes([7; 32]),
        terms,
        deposit: 245_280,
    };
    hex(&encode_ix(&ix).expect("encodage"))
}

#[test]
fn open_bytes_match_the_shared_vector() {
    let bytes = open_bytes();
    if std::env::var_os("SIL_REGENERATE_VECTORS").is_some() {
        std::fs::write(VECTOR, format!("{bytes}\n")).expect("écriture du vecteur");
    }
    let expected = std::fs::read_to_string(VECTOR).expect("vecteur présent");
    assert_eq!(
        bytes,
        expected.trim(),
        "format d'instruction Open changé : régénérer le vecteur"
    );
}
