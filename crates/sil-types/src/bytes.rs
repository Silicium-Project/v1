//! Octets de taille fixe : sérialisés comme une chaîne d'octets (CBOR `bstr`).

use crate::{TypeError, hex};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;

macro_rules! fixed_bytes {
    ($(#[$meta:meta])* $name:ident, $len:expr) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name([u8; $len]);

        impl $name {
            /// Longueur en octets.
            pub const LEN: usize = $len;

            /// Construit depuis un tableau.
            pub const fn from_bytes(bytes: [u8; $len]) -> Self {
                Self(bytes)
            }

            /// Octets bruts.
            pub const fn as_bytes(&self) -> &[u8; $len] {
                &self.0
            }

            /// Parse du hex minuscule strict.
            pub fn from_hex(text: &str) -> Result<Self, TypeError> {
                hex::decode_fixed::<$len>(text)
                    .map(Self)
                    .ok_or(TypeError::InvalidHex { kind: stringify!($name) })
            }

            /// Hex minuscule.
            pub fn to_hex(&self) -> String {
                hex::encode(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.to_hex())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.to_hex())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                if serializer.is_human_readable() {
                    serializer.serialize_str(&self.to_hex())
                } else {
                    serializer.serialize_bytes(&self.0)
                }
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct Visitor;
                impl de::Visitor<'_> for Visitor {
                    type Value = $name;

                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, "{} octets (hex de {} caractères en format texte)", $len, $len * 2)
                    }

                    fn visit_str<E: de::Error>(self, value: &str) -> Result<$name, E> {
                        $name::from_hex(value).map_err(|_| E::invalid_value(de::Unexpected::Str(value), &self))
                    }

                    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<$name, E> {
                        <[u8; $len]>::try_from(value)
                            .map($name)
                            .map_err(|_| E::invalid_length(value.len(), &self))
                    }
                }
                if deserializer.is_human_readable() {
                    deserializer.deserialize_str(Visitor)
                } else {
                    deserializer.deserialize_bytes(Visitor)
                }
            }
        }
    };
}

fixed_bytes!(
    /// Empreinte SHA-256.
    Hash32, 32
);
fixed_bytes!(
    /// Identifiant d'un job : `H(JobSpec)` (le même côté P2P et côté chaîne).
    JobId, 32
);
fixed_bytes!(
    /// Clé publique Ed25519.
    PublicKey, 32
);
fixed_bytes!(
    /// Signature Ed25519.
    Signature, 64
);
fixed_bytes!(
    /// Identifiant d'un contenu : racine Merkle liée à la taille (voir `sil-content`).
    ContentId, 32
);
