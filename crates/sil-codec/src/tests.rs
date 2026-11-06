use crate::{CodecError, decode, encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Sample {
    id: u32,
    name: String,
}

fn sample() -> Sample {
    Sample {
        id: 7,
        name: "job".into(),
    }
}

#[test]
fn round_trip() {
    let bytes = encode(&sample()).unwrap();
    assert_eq!(decode::<Sample>(&bytes, 1024).unwrap(), sample());
}

#[test]
fn encoding_is_stable() {
    assert_eq!(encode(&sample()).unwrap(), encode(&sample()).unwrap());
}

#[test]
fn oversized_input_is_refused_before_parsing() {
    let bytes = encode(&sample()).unwrap();
    assert!(matches!(
        decode::<Sample>(&bytes, 3),
        Err(CodecError::TooLarge { .. })
    ));
}

#[test]
fn trailing_bytes_are_refused() {
    let mut bytes = encode(&sample()).unwrap();
    bytes.push(0);
    assert_eq!(
        decode::<Sample>(&bytes, 1024),
        Err(CodecError::TrailingBytes)
    );
}

#[test]
fn non_minimal_integers_are_refused() {
    // map(2) {"id": 7 écrit sur 2 octets (0x18 0x07 au lieu de 0x07), "name": "job"}
    let mut bytes = vec![0xa2, 0x62, b'i', b'd', 0x18, 0x07];
    bytes.extend_from_slice(&[0x64, b'n', b'a', b'm', b'e', 0x63, b'j', b'o', b'b']);
    assert_eq!(
        decode::<Sample>(&bytes, 1024),
        Err(CodecError::NotCanonical)
    );
}

#[test]
fn reordered_fields_are_refused() {
    let mut bytes = vec![0xa2, 0x64, b'n', b'a', b'm', b'e', 0x63, b'j', b'o', b'b'];
    bytes.extend_from_slice(&[0x62, b'i', b'd', 0x07]);
    assert_eq!(
        decode::<Sample>(&bytes, 1024),
        Err(CodecError::NotCanonical)
    );
}

#[test]
fn deep_nesting_is_refused() {
    let bytes = vec![0x81; 200];
    assert!(decode::<Vec<Vec<u8>>>(&bytes, 1024).is_err());
}
