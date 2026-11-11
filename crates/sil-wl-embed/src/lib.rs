//! Workload d'embedding (docs/technical/embedding-workload.md).
//!
//! Ce module ne contient que le calcul pur après l'inférence : pooling moyen masqué et
//! normalisation L2. L'inférence ONNX est dans `engine`.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod engine;
mod workload;

/// Source de secours du modèle et de son tokenizer (hébergement Hugging Face, fichiers vérifiés).
pub const MODEL_URL: &str =
    "https://huggingface.co/intfloat/multilingual-e5-small/resolve/main/onnx/model.onnx";
pub const TOKENIZER_URL: &str =
    "https://huggingface.co/intfloat/multilingual-e5-small/resolve/main/tokenizer.json";
/// SHA-256 du tokenizer figé : il fait partie du résultat (un autre tokenizer, d'autres vecteurs).
pub const TOKENIZER_SHA256: &str =
    "0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39";

pub use engine::{EmbedError, Engine, MAX_TOKENS, PASSAGE_PREFIX};
pub use workload::{EmbedWorkload, decode_vector, encode_vectors};

/// Dimension des vecteurs de multilingual-e5-small.
pub const DIM: usize = 384;

/// Moyenne des tokens non masqués. `hidden` : `tokens × dim` en ordre de ligne ; `mask` : 1 pour
/// un token réel, 0 pour le remplissage. Sans token réel, le vecteur est nul (jamais de division
/// par zéro).
pub fn mean_pool(hidden: &[f32], mask: &[i64], dim: usize) -> Vec<f32> {
    let mut pooled = vec![0f32; dim];
    let mut count = 0f32;
    for (row, &keep) in mask.iter().enumerate() {
        if keep == 0 {
            continue;
        }
        count += 1.0;
        let start = row * dim;
        for (acc, value) in pooled.iter_mut().zip(&hidden[start..start + dim]) {
            *acc += value;
        }
    }
    if count > 0.0 {
        for acc in &mut pooled {
            *acc /= count;
        }
    }
    pooled
}

/// Normalisation L2 en place. Un vecteur nul reste nul.
pub fn l2_normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector.iter_mut() {
            *value /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_does_not_change_the_mean() {
        // Deux tokens réels de dimension 2, puis un token de remplissage énorme.
        let hidden = [1.0f32, 3.0, 5.0, 7.0, 1000.0, 1000.0];
        let mask = [1i64, 1, 0];
        assert_eq!(mean_pool(&hidden, &mask, 2), vec![3.0, 5.0]);
    }

    #[test]
    fn no_real_token_gives_a_zero_vector() {
        let hidden = [1.0f32, 2.0];
        assert_eq!(mean_pool(&hidden, &[0], 2), vec![0.0, 0.0]);
    }

    #[test]
    fn normalisation_gives_unit_length() {
        let mut v = vec![3.0f32, 4.0];
        l2_normalize(&mut v);
        assert!((v[0] - 0.6).abs() < 1e-6 && (v[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn a_zero_vector_stays_zero_after_normalisation() {
        let mut v = vec![0.0f32, 0.0];
        l2_normalize(&mut v);
        assert_eq!(v, vec![0.0, 0.0]);
    }
}
