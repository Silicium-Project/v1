use crate::{Hash32, Signature, hex};

#[test]
fn hex_round_trip() {
    let hash = Hash32::from_bytes([0xab; 32]);
    assert_eq!(Hash32::from_hex(&hash.to_hex()).unwrap(), hash);
}

#[test]
fn hex_is_strict() {
    assert!(Hash32::from_hex(&"AB".repeat(32)).is_err(), "majuscules");
    assert!(Hash32::from_hex(&"ab".repeat(31)).is_err(), "trop court");
    assert!(Hash32::from_hex(&format!("{}zz", "ab".repeat(31))).is_err());
    assert!(hex::decode_fixed::<2>("abc").is_none());
}

#[test]
fn cbor_uses_byte_strings_and_checks_length() {
    let sig = Signature::from_bytes([7; 64]);
    let mut buf = Vec::new();
    ciborium::into_writer(&sig, &mut buf).unwrap();
    assert_eq!(buf[0], 0x58, "bstr avec longueur sur 1 octet");
    let back: Signature = ciborium::from_reader(buf.as_slice()).unwrap();
    assert_eq!(back, sig);
    buf.pop();
    assert!(ciborium::from_reader::<Signature, _>(buf.as_slice()).is_err());
}

#[test]
fn cbor_stays_compact_bytes_not_hex_text() {
    let mut buf = Vec::new();
    ciborium::into_writer(&Hash32::from_bytes([0xab; 32]), &mut buf).unwrap();
    assert_eq!(
        buf.len(),
        2 + 32,
        "bstr de 32 octets, pas 64 caractères hex"
    );
}
