//! Workload `image/batch` : redimensionnement et réencodage d'un lot d'images (conteneur
//! SILBATCH, voir `sil-batch`). Chaque fragment est une plage d'images ; un vérificateur refait
//! le traitement de la plage (politique `Recompute`). Le pilote est en Rust pur : même version de
//! l'encodeur, mêmes octets.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod codec;
mod transform;
mod workload;

pub use codec::{FORMAT_JPEG, FORMAT_PNG};
pub use workload::ImageBatchWorkload;

#[cfg(test)]
mod tests;
