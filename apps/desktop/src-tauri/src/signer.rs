//! Machine key and device-link proofs.
//!
//! The key is created here, stored in the app data folder (mode 0600 on Unix) and never sent
//! to the web UI: the UI only receives the public key and signatures. Errors are stable codes
//! (`no_key`, `bad_code`, `storage`) that the UI translates; no path or secret is ever returned.

use serde::Serialize;
use sil_crypto::{Domain, Purpose, keyfile};
use std::path::{Path, PathBuf};

/// Length of a link code (matches the API, services/api/internal/devices).
pub const CODE_LEN: usize = 12;
/// Alphabet of link codes: no 0/O/1/I, so a code read aloud or copied is not ambiguous.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerError {
    NoKey,
    BadCode,
    Storage,
}

impl SignerError {
    /// Stable code sent to the UI.
    pub fn code(self) -> &'static str {
        match self {
            Self::NoKey => "no_key",
            Self::BadCode => "bad_code",
            Self::Storage => "storage",
        }
    }
}

/// What the UI needs to link this machine: the public key and the signature of the code.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LinkProof {
    pub public_key: String,
    pub signature: String,
}

/// Owns the location of the machine key.
#[derive(Debug, Clone)]
pub struct KeyStore {
    path: PathBuf,
}

impl KeyStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Public key in hex, or None when no key exists yet.
    pub fn public_key(&self) -> Result<Option<String>, SignerError> {
        if !self.path.exists() {
            return Ok(None);
        }
        Ok(Some(identity_of(&self.path)?.public().to_hex()))
    }

    /// Creates the key if it does not exist (never overwrites) and returns its public key.
    pub fn create(&self) -> Result<String, SignerError> {
        keyfile::load_or_create(&self.path).map_err(|_| SignerError::Storage)?;
        self.public_key()?.ok_or(SignerError::Storage)
    }

    /// Signs a link code with the machine key, under the device-link domain.
    pub fn link_proof(&self, code: &str) -> Result<LinkProof, SignerError> {
        let code = normalize_code(code)?;
        if !self.path.exists() {
            return Err(SignerError::NoKey);
        }
        let identity = identity_of(&self.path)?;
        let public = identity.public();
        // Same bytes the API checks: code (upper case) || raw public key.
        let mut message = code.into_bytes();
        message.extend_from_slice(public.as_bytes());
        let signature = identity.sign(Domain::DeviceLink, &message);
        Ok(LinkProof {
            public_key: public.to_hex(),
            signature: signature.to_hex(),
        })
    }
}

fn identity_of(path: &Path) -> Result<sil_crypto::SigningIdentity, SignerError> {
    let seed = keyfile::load(path).map_err(|_| SignerError::Storage)?;
    Ok(seed.derive(Purpose::Ledger))
}

/// Upper-cases and checks a link code. Returns the canonical bytes-as-text.
pub fn normalize_code(code: &str) -> Result<String, SignerError> {
    let upper = code.trim().to_ascii_uppercase();
    let valid = upper.len() == CODE_LEN && upper.bytes().all(|b| CODE_ALPHABET.contains(&b));
    if valid {
        Ok(upper)
    } else {
        Err(SignerError::BadCode)
    }
}

/// Checks a proof the way the API does. Only the tests use it: the API is the real verifier.
#[cfg(test)]
pub fn verify_proof(proof: &LinkProof, code: &str) -> bool {
    use sil_types::{PublicKey, Signature};
    let (Ok(public), Ok(signature)) = (
        PublicKey::from_hex(&proof.public_key),
        Signature::from_hex(&proof.signature),
    ) else {
        return false;
    };
    let mut message = code.to_ascii_uppercase().into_bytes();
    message.extend_from_slice(public.as_bytes());
    sil_crypto::verify(&public, Domain::DeviceLink, &message, &signature).is_ok()
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "tests fail loudly on purpose")]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, KeyStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = KeyStore::new(dir.path().join("device.key"));
        (dir, store)
    }

    #[test]
    fn no_key_until_one_is_created() {
        let (_dir, store) = store();
        assert_eq!(store.public_key(), Ok(None));
        assert_eq!(store.link_proof("ABCDEFGHJKLM"), Err(SignerError::NoKey));
    }

    #[test]
    fn created_key_gives_a_proof_the_api_rule_accepts() {
        let (_dir, store) = store();
        let public = store.create().expect("key created");
        let proof = store.link_proof("abcdefghjklm").expect("proof");
        assert_eq!(proof.public_key, public);
        assert!(verify_proof(&proof, "ABCDEFGHJKLM"));
    }

    #[test]
    fn a_proof_is_bound_to_its_code() {
        let (_dir, store) = store();
        store.create().expect("key created");
        let proof = store.link_proof("ABCDEFGHJKLM").expect("proof");
        assert!(!verify_proof(&proof, "ZZZZZZZZZZZZ"));
    }

    #[test]
    fn create_never_replaces_an_existing_key() {
        let (_dir, store) = store();
        let first = store.create().expect("first");
        let second = store.create().expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn malformed_codes_are_refused_before_touching_the_key() {
        for bad in [
            "",
            "ABC",
            "ABCDEFGHJKL0",
            "ABCDEFGHJKLI",
            "ABCDEFGHJKL!",
            "ABCDEFGHJKLMN",
        ] {
            assert_eq!(normalize_code(bad), Err(SignerError::BadCode), "{bad:?}");
        }
    }

    #[test]
    fn error_codes_are_stable_strings() {
        assert_eq!(SignerError::NoKey.code(), "no_key");
        assert_eq!(SignerError::BadCode.code(), "bad_code");
        assert_eq!(SignerError::Storage.code(), "storage");
    }
}
