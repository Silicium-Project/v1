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
