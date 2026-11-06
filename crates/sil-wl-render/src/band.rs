//! Entrée (en-tête + scène) et bande de lignes d'un fragment.

use serde::{Deserialize, Serialize};
use sil_codec::{decode, encode};
use sil_workload_api::{FragmentSpec, InputReader, WorkloadError};

/// Dimensions maximales acceptées (évite les allocations démesurées).
const MAX_SIDE: u32 = 16_384;
const HEADER: usize = 8;

/// En-tête de l'entrée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Header {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Header {
    pub(crate) fn read(input: &dyn InputReader) -> Result<Self, WorkloadError> {
        if input.len() < HEADER as u64 {
            return Err(WorkloadError::BadInput("en-tête manquant".into()));
        }
        let bytes = input.read(0, HEADER)?;
        let width = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let height = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
            return Err(WorkloadError::BadInput("dimensions".into()));
        }
        Ok(Self { width, height })
    }

    /// La scène : tout ce qui suit l'en-tête.
    pub(crate) fn scene(input: &dyn InputReader) -> Result<Vec<u8>, WorkloadError> {
        let len = usize::try_from(input.len() - HEADER as u64)
            .map_err(|_| WorkloadError::BadInput("scène trop grande".into()))?;
        input.read(HEADER as u64, len)
    }
}

/// Lignes `[y_start, y_end)` à rendre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Band {
    pub(crate) y_start: u32,
    pub(crate) y_end: u32,
}

impl Band {
    pub(crate) fn to_spec(self, index: u32) -> Result<FragmentSpec, WorkloadError> {
        let params = encode(&self).map_err(|e| WorkloadError::BadSpec(e.to_string()))?;
        Ok(FragmentSpec { index, params })
    }

    pub(crate) fn from_spec(spec: &FragmentSpec, height: u32) -> Result<Self, WorkloadError> {
        let band: Self = decode(&spec.params, 32)
            .map_err(|e| WorkloadError::BadSpec(format!("bande illisible : {e}")))?;
        if band.y_start >= band.y_end || band.y_end > height {
            return Err(WorkloadError::BadSpec("bande hors de l'image".into()));
        }
        Ok(band)
    }

    pub(crate) fn rows(self) -> u32 {
        self.y_end - self.y_start
    }
}
