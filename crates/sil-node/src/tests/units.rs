//! Les unités du contrat (`sil_chain_core::units`) coïncident avec le découpage réel des
//! workloads. Sinon le worker et l'escrow compteraient des montants différents.

use serde::{Deserialize, Serialize};
use sil_chain_core::units::{RENDER_WORKLOAD, SHA256_WORKLOAD, Shape, fragment_units};
use sil_codec::decode;
use sil_wl_render::{BlockRenderer, RenderWorkload};
use sil_wl_sha256::Sha256Workload;
use sil_workload_api::{MemoryInput, Workload, WorkloadError};

/// Même forme que la plage du workload `calcul/sha256` (type privé dans `sil-wl-sha256`).
#[derive(Serialize, Deserialize)]
struct Plage {
    start: u64,
    end: u64,
}

/// Même forme que la bande du workload `rendu/blocs` (type privé dans `sil-wl-render`).
#[derive(Serialize, Deserialize)]
struct Bande {
    y_start: u32,
    y_end: u32,
}

/// Le découpage ne rend rien : seule la forme des bandes nous intéresse.
struct NoRender;

impl BlockRenderer for NoRender {
    fn render(
        &self,
        _scene: &[u8],
        _width: u32,
        _height: u32,
        _y0: u32,
        _y1: u32,
    ) -> Result<Vec<u8>, WorkloadError> {
        Ok(Vec::new())
    }
}

#[test]
fn the_unit_rule_matches_the_sha256_split() {
    for len in [0usize, 1, 2, 3, 7, 10, 31, 20_000] {
        let bytes = vec![0u8; len];
        let input = MemoryInput(&bytes);
        for fragments in [1u32, 2, 3, 5, 16] {
            let specs = Sha256Workload.split(&input, fragments).unwrap();
            let sizes: Vec<u64> = specs
                .iter()
                .map(|spec| {
                    let plage: Plage = decode(&spec.params, 64).unwrap();
                    plage.end - plage.start
                })
                .collect();
            let shape = Shape {
                input_size: len as u64,
                width: 0,
                height: 0,
            };
            let units: Result<Vec<u64>, _> = (0..fragments)
                .map(|index| fragment_units(SHA256_WORKLOAD, shape, fragments, index))
                .collect();
            if specs.len() == fragments as usize {
                assert_eq!(units.unwrap(), sizes, "entrée {len}, fragments {fragments}");
            } else {
                // Le workload ferait moins de fragments que prévu : le job est refusé à l'ouverture.
                assert!(units.is_err(), "entrée {len}, fragments {fragments}");
            }
        }
    }
}

#[test]
fn the_unit_rule_matches_the_render_split() {
    let workload = RenderWorkload::new(NoRender);
    for (width, height) in [(1u32, 1u32), (4, 8), (7, 20), (3, 24), (64, 37), (2, 40)] {
        // En-tête (largeur, hauteur) puis une petite scène.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3, 4]);
        let input = MemoryInput(&bytes);
        let blocks = height.div_ceil(8);
        for fragments in 1..=blocks + 1 {
            let specs = workload.split(&input, fragments).unwrap();
            let rows: Vec<u64> = specs
                .iter()
                .map(|spec| {
                    let bande: Bande = decode(&spec.params, 32).unwrap();
                    u64::from(bande.y_end - bande.y_start)
                })
                .collect();
            let shape = Shape {
                input_size: bytes.len() as u64,
                width,
                height,
            };
            let units: Result<Vec<u64>, _> = (0..fragments)
                .map(|index| fragment_units(RENDER_WORKLOAD, shape, fragments, index))
                .collect();
            if specs.len() == fragments as usize {
                let expected: Vec<u64> = rows.iter().map(|r| r * u64::from(width)).collect();
                assert_eq!(
                    units.unwrap(),
                    expected,
                    "{width}x{height}, fragments {fragments}"
                );
            } else {
                assert!(units.is_err(), "{width}x{height}, fragments {fragments}");
            }
        }
    }
}
