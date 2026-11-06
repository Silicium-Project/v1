use serde::{Deserialize, Serialize};
use sil_codec::{decode, encode};
use sil_workload_api::{FragmentSpec, WorkloadError};

/// Plage `[start, end)` d'octets de l'entrée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Range {
    pub(crate) start: u64,
    pub(crate) end: u64,
}

impl Range {
    pub(crate) fn to_spec(self, index: u32) -> Result<FragmentSpec, WorkloadError> {
        let params = encode(&self).map_err(|e| WorkloadError::BadSpec(e.to_string()))?;
        Ok(FragmentSpec { index, params })
    }

    /// Décode et contrôle la plage contre la longueur de l'entrée.
    pub(crate) fn from_spec(spec: &FragmentSpec, input_len: u64) -> Result<Self, WorkloadError> {
        let range: Self = decode(&spec.params, 64)
            .map_err(|e| WorkloadError::BadSpec(format!("plage illisible : {e}")))?;
        if range.start > range.end || range.end > input_len {
            return Err(WorkloadError::BadSpec("plage hors de l'entrée".into()));
        }
        Ok(range)
    }
}
