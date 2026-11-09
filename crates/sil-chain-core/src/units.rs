//! Unités de travail : combien vaut chaque fragment d'un job.
//!
//! Les tarifs du job sont **par unité** (`reward_per_unit`, `verifier_reward_per_unit`). Un
//! fragment rapporte donc `unités × tarif`. Les unités sont fixées par la spec signée (le
//! workload, la taille de l'entrée et l'image), jamais par le comité ni par le worker.
//!
//! Le type d'unité (`sil_workload_spec::Unit`) et la géométrie viennent du descripteur du workload
//! (`contracts/workloads`). Ici : une règle par **type** d'unité, pas par workload. Un workload de
//! type connu ne demande donc aucune modification on-chain ; un type nouveau en demande une.
//! Les règles de découpage doivent rester celles des workloads : `sil-node` contrôle cette égalité
//! par des tests. Un workload absent des descripteurs ne peut pas être réglé on-chain.

use crate::RuleError;
use sil_workload_spec::{Spec, Unit};

/// Forme de l'entrée, telle que la spec la déclare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    /// Taille de l'entrée, en octets.
    pub input_size: u64,
    /// Largeur de l'image en pixels (`rendu/raytracer`).
    pub width: u32,
    /// Hauteur de l'image en pixels (`rendu/raytracer`).
    pub height: u32,
}

/// Unités du fragment `index` (à partir de 0) d'un job de `fragments` fragments.
pub fn fragment_units(
    workload: &str,
    shape: Shape,
    fragments: u32,
    index: u32,
) -> Result<u64, RuleError> {
    if index >= fragments {
        return Err(RuleError::UnknownFragment);
    }
    let spec = sil_workload_spec::find(workload)
        .ok_or(RuleError::BadTerms("workload sans descripteur"))?;
    match spec.unit {
        Unit::BytesInput => bytes_units(shape.input_size, fragments, index),
        Unit::PixelsBand => pixels_band_units(spec, shape, fragments, index),
    }
}

/// Découpage par plages d'octets, comme `sil-wl-sha256` : `count = min(fragments, max(len, 1))`,
/// et les `len % count` premiers fragments ont un octet de plus.
/// Un découpage plus fin que l'entrée ferait produire moins de fragments que prévu : refusé.
fn bytes_units(input_size: u64, fragments: u32, index: u32) -> Result<u64, RuleError> {
    let count = u64::from(fragments);
    if count > input_size.max(1) {
        return Err(RuleError::BadTerms(
            "entrée plus petite que le nombre de fragments",
        ));
    }
    let base = input_size / count;
    let extra = input_size % count;
    Ok(base + u64::from(u64::from(index) < extra))
}

/// Découpage par bandes de blocs de lignes (`block_rows` du descripteur) : les
/// `blocks = ceil(hauteur / block_rows)` blocs sont répartis sur les fragments (les premiers ont un
/// bloc de plus). Une bande vaut sa largeur fois ses lignes ; la dernière est coupée à la hauteur.
fn pixels_band_units(
    spec: &Spec,
    shape: Shape,
    fragments: u32,
    index: u32,
) -> Result<u64, RuleError> {
    let (width, height) = (u64::from(shape.width), u64::from(shape.height));
    let block_rows = u64::from(spec.block_rows);
    if width == 0
        || height == 0
        || width > u64::from(spec.max_side)
        || height > u64::from(spec.max_side)
    {
        return Err(RuleError::BadTerms("dimensions de l'image"));
    }
    let blocks = height.div_ceil(block_rows);
    let count = u64::from(fragments);
    if count > blocks {
        return Err(RuleError::BadTerms(
            "image plus petite que le nombre de fragments",
        ));
    }
    let (base, extra) = (blocks / count, blocks % count);
    let index = u64::from(index);
    let first = index * base + index.min(extra);
    let last = first + base + u64::from(index < extra);
    let rows = (last * block_rows).min(height) - first * block_rows;
    Ok(width * rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_workload_spec::{CALCUL_SHA256, RENDU_RAYTRACER};

    const SHA256_WORKLOAD: &str = CALCUL_SHA256.id;
    const RENDER_WORKLOAD: &str = RENDU_RAYTRACER.id;
    use crate::{Escrow, EscrowTerms, Payout, SETTLEMENT_FEE, SettleArgs, reputation};
    use sil_types::{Hash32, JobId, PublicKey};

    fn key(n: u8) -> PublicKey {
        PublicKey::from_bytes([n; 32])
    }

    fn sha(input_size: u64) -> Shape {
        Shape {
            input_size,
            width: 0,
            height: 0,
        }
    }

    fn image(width: u32, height: u32) -> Shape {
        Shape {
            input_size: 0,
            width,
            height,
        }
    }

    #[test]
    fn payouts_follow_each_fragments_units() {
        // 31 octets sur 3 fragments : 11 + 10 + 10 unités. Le premier fragment paie 11 unités.
        let terms = EscrowTerms {
            requester: key(100),
            workload: SHA256_WORKLOAD.into(),
            input_size: 31,
            width: 0,
            height: 0,
            fragments: 3,
            verifications_required: 2,
            reward_per_unit: 10,
            verifier_reward_per_unit: 1,
            expires_at_ms: 1_000_000,
        };
        let deposit = terms.max_payout().unwrap();
        let mut escrow =
            Escrow::open(JobId::from_bytes([1; 32]), terms, deposit, &key(100)).unwrap();
        let args = SettleArgs {
            fragment: 0,
            attempt: 1,
            result: Hash32::from_bytes([7; 32]),
            claimant: key(60),
            approvers: vec![key(70), key(71)],
        };
        let payouts = escrow
            .settle(
                &args,
                &[key(70), key(71)],
                &key(200),
                &[Some(reputation::VERIFIER_START); 2],
            )
            .unwrap()
            .payouts;
        assert_eq!(
            payouts[0],
            Payout {
                to: key(60),
                amount: 110
            }
        );
        assert_eq!(payouts[1].amount, 11, "chaque vérificateur : 11 × 1");
        assert_eq!(payouts[2].amount, 11);
        assert_eq!(
            payouts[3],
            Payout {
                to: key(200),
                amount: SETTLEMENT_FEE
            }
        );
        assert_eq!(escrow.paid(), 11 * 12 + SETTLEMENT_FEE);
    }

    #[test]
    fn sha256_units_follow_the_workload_split() {
        // 10 octets sur 3 fragments : 4 + 3 + 3, comme le découpage de sil-wl-sha256.
        let units: Vec<u64> = (0..3)
            .map(|i| fragment_units(SHA256_WORKLOAD, sha(10), 3, i).unwrap())
            .collect();
        assert_eq!(units, [4, 3, 3]);
        assert_eq!(units.iter().sum::<u64>(), 10, "toute l'entrée est comptée");
    }

    #[test]
    fn an_input_smaller_than_the_fragment_count_is_refused() {
        assert!(matches!(
            fragment_units(SHA256_WORKLOAD, sha(2), 3, 0),
            Err(RuleError::BadTerms(_))
        ));
    }

    #[test]
    fn an_empty_input_is_one_fragment_of_no_units() {
        assert_eq!(fragment_units(SHA256_WORKLOAD, sha(0), 1, 0), Ok(0));
        assert!(fragment_units(SHA256_WORKLOAD, sha(0), 2, 0).is_err());
    }

    #[test]
    fn render_units_follow_the_band_split() {
        // 16 × 24 sur 2 fragments : 3 blocs, donc 2 blocs (lignes 0..16) puis 1 bloc (16..24).
        let units: Vec<u64> = (0..2)
            .map(|i| fragment_units(RENDER_WORKLOAD, image(16, 24), 2, i).unwrap())
            .collect();
        assert_eq!(units, [16 * 16, 16 * 8]);
    }

    #[test]
    fn the_last_render_band_is_cut_at_the_image_height() {
        // 10 × 20 sur 3 fragments : 3 blocs (8, 8, 4 lignes).
        let units: Vec<u64> = (0..3)
            .map(|i| fragment_units(RENDER_WORKLOAD, image(10, 20), 3, i).unwrap())
            .collect();
        assert_eq!(units, [80, 80, 40]);
    }

    #[test]
    fn render_units_always_add_up_to_the_pixels() {
        for height in 1..=40u32 {
            for width in [1u32, 7, 64] {
                let blocks = height.div_ceil(8);
                for fragments in 1..=blocks {
                    let total: u64 = (0..fragments)
                        .map(|i| {
                            fragment_units(RENDER_WORKLOAD, image(width, height), fragments, i)
                                .unwrap()
                        })
                        .sum();
                    assert_eq!(
                        total,
                        u64::from(width) * u64::from(height),
                        "{width}x{height}/{fragments}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_render_image_with_fewer_blocks_than_fragments_is_refused() {
        assert!(matches!(
            fragment_units(RENDER_WORKLOAD, image(16, 8), 2, 0),
            Err(RuleError::BadTerms(_))
        ));
        assert!(matches!(
            fragment_units(RENDER_WORKLOAD, image(0, 8), 1, 0),
            Err(RuleError::BadTerms(_))
        ));
    }

    #[test]
    fn workloads_without_a_unit_rule_are_refused() {
        assert!(matches!(
            fragment_units("autre/workload", image(16, 16), 1, 0),
            Err(RuleError::BadTerms(_))
        ));
    }

    #[test]
    fn an_index_past_the_fragments_is_unknown() {
        assert_eq!(
            fragment_units(SHA256_WORKLOAD, sha(10), 3, 3),
            Err(RuleError::UnknownFragment)
        );
    }
}
