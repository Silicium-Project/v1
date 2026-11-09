//! Workload de rendu par blocs de lignes (`rendu/raytracer`).
//!
//! Le moteur de rendu lui-même est injecté ([`BlockRenderer`]) : ce crate porte la logique
//! de **découpe, d'engagement et de vérification**, pas un lanceur de rayons. En production le
//! moteur tourne dans `sil-sandbox` ; dans les tests c'est une fonction déterministe.
//!
//! Format d'entrée : `largeur u32 LE ‖ hauteur u32 LE ‖ scène`. Sortie d'un fragment : les
//! pixels RGB24 de ses lignes, et un hash par bloc de 8 lignes (les *milestones*).
//!
//! La vérification fait trois contrôles (V0 n'en faisait qu'un, sur l'échantillon) :
//! 1. taille et nombre de milestones cohérents ;
//! 2. **chaque bloc de pixels livré correspond à son milestone** (sinon on pourrait engager
//!    des hash honnêtes et livrer d'autres pixels) ;
//! 3. les blocs de l'échantillon sont **recalculés** et comparés aux milestones.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod band;
mod external;
mod reference;
mod renderer;
mod verify;
mod workload;

pub use external::ExternalRenderer;
pub use reference::reference_pixels;
pub use renderer::BlockRenderer;
pub use workload::{ROWS_PER_BLOCK, RenderWorkload};

#[cfg(test)]
mod tests;
