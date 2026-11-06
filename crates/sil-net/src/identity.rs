use crate::NetError;
use libp2p::identity::Keypair;
use sil_crypto::SecretBytes;

/// Identité libp2p issue de la clé `Purpose::Network` de la graine du nœud
/// (clé distincte de celle du journal, voir docs/technical/security/keys.md).
pub fn keypair_from_secret(secret: &SecretBytes) -> Result<Keypair, NetError> {
    let mut bytes = *secret.expose();
    let keypair =
        Keypair::ed25519_from_bytes(&mut bytes).map_err(|e| NetError::Config(e.to_string()))?;
    bytes.fill(0);
    Ok(keypair)
}
