use zeroize::ZeroizeOnDrop;

/// Octets secrets bruts, effacés à la destruction. Réservé aux adaptateurs de transport
/// (ex. identité libp2p) qui exigent la clé brute ; ne pas journaliser, ne pas copier.
#[derive(ZeroizeOnDrop)]
pub struct SecretBytes([u8; 32]);

impl SecretBytes {
    pub(crate) fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Accès aux octets, pour les consommer immédiatement.
    pub fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretBytes(<secret>)")
    }
}
