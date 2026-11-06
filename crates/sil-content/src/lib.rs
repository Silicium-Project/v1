//! Contenu adressé par hash, découpé en morceaux de 256 Kio vérifiables un par un.
//!
//! Corrige V0 : `has` lisait et hashait le fichier entier (jusqu'à 64 Mio) à chaque appel,
//! et un téléchargement n'était vérifié qu'à la fin. Ici :
//! - l'**identifiant** est une racine de Merkle liée à la taille ([`merkle::content_id`]) ;
//! - le **manifeste** liste le hash de chaque morceau (il se vérifie contre l'identifiant) ;
//! - chaque morceau reçu est vérifié **avant** d'être écrit, donc une source malveillante
//!   est détectée au premier morceau faux et le reste est reprenable auprès d'un autre pair ;
//! - `has` est une recherche en mémoire, `chunk` ne lit que le morceau demandé.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod config;
mod download;
mod error;
mod index;
mod manifest;
pub mod merkle;
mod paths;
mod store;

pub use config::StoreConfig;
pub use download::Download;
pub use error::ContentError;
pub use manifest::Manifest;
pub use merkle::CHUNK_SIZE;
pub use store::ContentStore;

#[cfg(test)]
mod tests;
