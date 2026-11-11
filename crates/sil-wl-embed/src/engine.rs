//! Moteur d'inférence : ONNX Runtime chargé dynamiquement, modèle fp32 figé, un texte à la fois.
//!
//! Un texte à la fois : pas de remplissage par lot, donc pas d'effet de bord entre textes. Un
//! thread : l'ordre des additions reste celui mesuré (docs/technical/embedding-workload.md, §9).

use crate::{DIM, l2_normalize, mean_pool};
use ort::{Session, Tensor};
use std::path::Path;
use thiserror::Error;
use tokenizers::Tokenizer;

/// Préfixe obligatoire pour multilingual-e5-small (y compris hors anglais).
pub const PASSAGE_PREFIX: &str = "passage: ";
/// Longueur maximale d'un chunk, en tokens.
pub const MAX_TOKENS: usize = 512;

/// Erreur du moteur d'embedding.
#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("runtime ONNX : {0}")]
    Runtime(String),
    #[error("tokenizer : {0}")]
    Tokenizer(String),
    #[error("forme de sortie inattendue")]
    Shape,
}

/// Moteur prêt à embedder.
pub struct Engine {
    session: Session,
    tokenizer: Tokenizer,
}

impl Engine {
    /// Ouvre le runtime (bibliothèque ONNX Runtime fournie), le modèle et le tokenizer.
    pub fn open(
        runtime_library: &Path,
        model: &Path,
        tokenizer_file: &Path,
    ) -> Result<Self, EmbedError> {
        ort::init_from(runtime_library.to_string_lossy().into_owned())
            .commit()
            .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        let session = Session::builder()
            .and_then(|builder| builder.with_intra_threads(1))
            .and_then(|builder| builder.commit_from_file(model))
            .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        let mut tokenizer = Tokenizer::from_file(tokenizer_file)
            .map_err(|error| EmbedError::Tokenizer(error.to_string()))?;
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length: MAX_TOKENS,
                ..Default::default()
            }))
            .map_err(|error| EmbedError::Tokenizer(error.to_string()))?;
        tokenizer.with_padding(None);
        Ok(Self { session, tokenizer })
    }

    /// Vecteur unitaire de 384 dimensions pour un texte.
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let encoding = self
            .tokenizer
            .encode(format!("{PASSAGE_PREFIX}{text}"), true)
            .map_err(|error| EmbedError::Tokenizer(error.to_string()))?;
        let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| i64::from(id)).collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&bit| i64::from(bit))
            .collect();
        let tokens = ids.len();
        let input_ids = Tensor::from_array(([1usize, tokens], ids))
            .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        let attention = Tensor::from_array(([1usize, tokens], mask.clone()))
            .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        // Le modèle attend aussi token_type_ids : une seule phrase, donc tout à zéro.
        let token_types = Tensor::from_array(([1usize, tokens], vec![0i64; tokens]))
            .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        let inputs = ort::inputs![
            "input_ids" => input_ids,
            "attention_mask" => attention,
            "token_type_ids" => token_types
        ]
        .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        let outputs = self
            .session
            .run(inputs)
            .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        let (shape, hidden) = outputs[0]
            .try_extract_raw_tensor::<f32>()
            .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        if shape.len() != 3 || shape[1] != tokens as i64 || shape[2] != DIM as i64 {
            return Err(EmbedError::Shape);
        }
        let mut vector = mean_pool(hidden, &mask, DIM);
        l2_normalize(&mut vector);
        Ok(vector)
    }
}
