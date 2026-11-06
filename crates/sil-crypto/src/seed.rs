use crate::{CryptoError, SecretBytes, SigningIdentity};
use ed25519_dalek::SigningKey;
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::ZeroizeOnDrop;

/// Usage d'une clé dérivée. Les clés de deux usages sont indépendantes
/// (V0 réutilisait la clé réseau pour le journal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Identité libp2p / authentification des pairs.
    Network,
    /// Signature des événements du journal.
    Ledger,
}

impl Purpose {
    const fn info(self) -> &'static [u8] {
        match self {
            Self::Network => b"sil/v1/key/network",
            Self::Ledger => b"sil/v1/key/ledger",
        }
    }
}

/// Graine maîtresse de 32 octets, effacée de la mémoire à la destruction.
#[derive(ZeroizeOnDrop)]
pub struct MasterSeed([u8; 32]);

impl MasterSeed {
    /// Tire une graine du générateur du système.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).map_err(|_| CryptoError::Random)?;
        Ok(Self(bytes))
    }

    /// Reconstruit depuis des octets (fichier clé, tests).
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    fn okm(&self, purpose: Purpose) -> [u8; 32] {
        let mut okm = [0u8; 32];
        // Une sortie de 32 octets ne dépasse jamais la limite de HKDF-SHA256.
        if Hkdf::<Sha256>::new(None, &self.0)
            .expand(purpose.info(), &mut okm)
            .is_err()
        {
            unreachable!("32 octets sont toujours dérivables");
        }
        okm
    }

    /// Dérive l'identité de signature d'un usage (HKDF-SHA256).
    pub fn derive(&self, purpose: Purpose) -> SigningIdentity {
        let mut okm = self.okm(purpose);
        let identity = SigningIdentity::from_key(SigningKey::from_bytes(&okm));
        okm.fill(0);
        identity
    }

    /// Octets bruts de la clé d'un usage, pour un adaptateur de transport (libp2p).
    /// La clé publique correspondante est celle de `derive(purpose).public()`.
    pub fn derive_raw(&self, purpose: Purpose) -> SecretBytes {
        SecretBytes::new(self.okm(purpose))
    }
}

impl std::fmt::Debug for MasterSeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MasterSeed(<secret>)")
    }
}
