use crate::{Domain, MasterSeed, Purpose, ct_eq, keyfile, sha256_domain, verify};

fn seed() -> MasterSeed {
    MasterSeed::from_bytes([9; 32])
}

#[test]
fn signature_round_trip() {
    let id = seed().derive(Purpose::Ledger);
    let sig = id.sign(Domain::Event, b"hello");
    assert!(verify(&id.public(), Domain::Event, b"hello", &sig).is_ok());
    assert!(verify(&id.public(), Domain::Event, b"hellp", &sig).is_err());
}

#[test]
fn a_signature_is_bound_to_its_domain() {
    let id = seed().derive(Purpose::Ledger);
    let sig = id.sign(Domain::Event, b"msg");
    assert!(verify(&id.public(), Domain::Attestation, b"msg", &sig).is_err());
}

#[test]
fn purposes_give_independent_keys() {
    let network = seed().derive(Purpose::Network).public();
    let ledger = seed().derive(Purpose::Ledger).public();
    assert_ne!(network, ledger);
    assert_eq!(network, seed().derive(Purpose::Network).public());
}

#[test]
fn hashes_are_domain_separated() {
    assert_ne!(
        sha256_domain(Domain::JobSpec, b"x"),
        sha256_domain(Domain::Event, b"x")
    );
}

#[test]
fn constant_time_compare() {
    assert!(ct_eq(b"token", b"token"));
    assert!(!ct_eq(b"token", b"tokeN"));
    assert!(!ct_eq(b"token", b"toke"));
}

#[test]
fn keyfile_is_created_once_and_reloaded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("seed.key");
    let first = keyfile::load_or_create(&path).unwrap();
    let again = keyfile::load_or_create(&path).unwrap();
    assert_eq!(
        first.derive(Purpose::Network).public(),
        again.derive(Purpose::Network).public()
    );
    assert!(!path.with_extension("tmp").exists());
}

#[test]
fn keyfile_with_wrong_length_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("seed.key");
    std::fs::write(&path, b"short").unwrap();
    assert!(keyfile::load(&path).is_err());
}

#[cfg(unix)]
#[test]
fn keyfile_is_private_and_open_permissions_are_refused() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("seed.key");
    keyfile::load_or_create(&path).unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(keyfile::load(&path).is_err());
}

#[test]
fn raw_secret_matches_the_derived_identity() {
    let seed = seed();
    let raw = seed.derive_raw(Purpose::Network);
    let key = ed25519_dalek::SigningKey::from_bytes(raw.expose());
    assert_eq!(
        key.verifying_key().to_bytes(),
        *seed.derive(Purpose::Network).public().as_bytes()
    );
    assert_eq!(format!("{raw:?}"), "SecretBytes(<secret>)");
}

/// Vecteur partagé avec le service Go (contracts/device-link.md).
#[test]
fn device_link_proof_from_the_go_service_verifies() {
    use sil_types::{PublicKey, Signature};
    let public =
        PublicKey::from_hex("ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c")
            .unwrap();
    let signature = Signature::from_hex(
        "146c5c2c78b2a2bf29904f13fe926374ee5f343034ae2fdc763943c45ed00e71\
         4a0b892b48ecab33df2c1ef8174511b9f8a24e7a70d45a10e4c899bbda83ae0b",
    )
    .unwrap();
    let mut message = b"ABCDEFGHJKLM".to_vec();
    message.extend_from_slice(public.as_bytes());
    assert!(verify(&public, Domain::DeviceLink, &message, &signature).is_ok());
    assert!(verify(&public, Domain::Event, &message, &signature).is_err());
}
