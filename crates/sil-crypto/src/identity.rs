use crate::{CryptoError, Domain};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use sil_types::{PublicKey, Signature};

/// Clé de signature Ed25519 (effacée à la destruction par ed25519-dalek).
pub struct SigningIdentity {
    key: SigningKey,
}

impl SigningIdentity {
    pub(crate) fn from_key(key: SigningKey) -> Self {
        Self { key }
    }

    /// Clé publique correspondante.
    pub fn public(&self) -> PublicKey {
        PublicKey::from_bytes(self.key.verifying_key().to_bytes())
    }

    /// Signe `message` dans `domain`.
    pub fn sign(&self, domain: Domain, message: &[u8]) -> Signature {
        Signature::from_bytes(self.key.sign(&domain.frame(message)).to_bytes())
    }

    /// Signs the exact bytes, with no domain prefix. Used for messages a wallet shows as text:
    /// such a message names its own domain in its first line.
    pub fn sign_raw(&self, message: &[u8]) -> Signature {
        Signature::from_bytes(self.key.sign(message).to_bytes())
    }
}

impl std::fmt::Debug for SigningIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SigningIdentity({})", self.public())
    }
}

/// Verifies a signature made with [`SigningIdentity::sign_raw`] (no domain prefix).
pub fn verify_raw(
    public: &PublicKey,
    message: &[u8],
    signature: &Signature,
) -> Result<(), CryptoError> {
    let key = VerifyingKey::from_bytes(public.as_bytes()).map_err(|_| CryptoError::InvalidKey)?;
    let signature = ed25519_dalek::Signature::from_bytes(signature.as_bytes());
    key.verify_strict(message, &signature)
        .map_err(|_| CryptoError::BadSignature)
}

/// Vérifie une signature (mode strict : refuse les clés faibles et les signatures malléables).
pub fn verify(
    public: &PublicKey,
    domain: Domain,
    message: &[u8],
    signature: &Signature,
) -> Result<(), CryptoError> {
    let key = VerifyingKey::from_bytes(public.as_bytes()).map_err(|_| CryptoError::InvalidKey)?;
    let signature = ed25519_dalek::Signature::from_bytes(signature.as_bytes());
    key.verify_strict(&domain.frame(message), &signature)
        .map_err(|_| CryptoError::BadSignature)
}
