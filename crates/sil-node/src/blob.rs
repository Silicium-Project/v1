use serde::{Deserialize, Serialize};
use sil_codec::{CodecError, decode, encode};
use sil_net::Bytes;
use sil_types::Hash32;
use sil_workload_api::Output;

/// Ce qui est stocké dans le magasin de contenu pour un résultat : les octets et les
/// milestones. Son `ContentId` est l'identifiant publié dans l'événement `Submit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultBlob {
    pub result: Bytes,
    pub milestones: Vec<Hash32>,
}

impl ResultBlob {
    pub fn from_output(output: &Output) -> Self {
        Self {
            result: Bytes(output.result.clone()),
            milestones: output.milestones.clone(),
        }
    }

    pub fn into_output(self) -> Output {
        Output {
            result: self.result.0,
            milestones: self.milestones,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, CodecError> {
        encode(self)
    }

    pub fn from_bytes(bytes: &[u8], max_len: usize) -> Result<Self, CodecError> {
        decode(bytes, max_len)
    }
}

#[cfg(test)]
mod tests {
    use super::ResultBlob;
    use sil_net::Bytes;
    use sil_types::Hash32;

    /// Un résultat de plus de 4 Kio (fragment de rendu) doit se relire : ciborium refusait les
    /// chaînes d'octets plus longues que son buffer de scratch.
    #[test]
    #[allow(clippy::expect_used)]
    fn large_result_roundtrips() {
        let blob = ResultBlob {
            result: Bytes(vec![9; 4608]),
            milestones: vec![
                Hash32::from_bytes([1; 32]),
                Hash32::from_bytes([2; 32]),
                Hash32::from_bytes([3; 32]),
            ],
        };
        let bytes = blob.to_bytes().expect("encode");
        assert_eq!(
            ResultBlob::from_bytes(&bytes, 1 << 20).expect("decode"),
            blob
        );
    }
}
