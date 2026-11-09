//! Moteur de rendu raytracing de V1, porté de `silicium-raytracer` (V0, moteur 2).
//!
//! Une scène `.cfg` (syntaxe à blocs `clé = valeur`) se rend en pixels RGB24. Le rendu est
//! **déterministe** : deux machines qui rendent la même bande produisent les mêmes octets, sinon
//! un vérificateur honnête rejetterait un résultat honnête. Les fonctions transcendantes viennent
//! de `libm` (Rust pur), pas de la libc de la machine.
//!
//! Chaque pixel ne dépend que de ses coordonnées et de la scène : [`render::render_rows`] rend une
//! bande de lignes, mono-thread. Le découpage en bandes et la vérification sont dans `sil-wl-render`.
//!
//! Hors de ce crate pour l'instant : la vérification de scène (`check`, à porter avec l'éditeur),
//! l'import `.obj`, et le pipeline de tuiles de V0 (remplacé par les bandes de `sil-wl-render`).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod check;
pub mod geometry;
pub mod integrator;
pub mod math;
pub mod obj;
pub mod random;
pub mod render;
pub mod scene;
pub mod syntax;

/// Version du moteur. Une scène qui demande une version plus récente est refusée, au lieu d'être
/// dessinée autrement par les nœuds qui la comprennent.
pub const ENGINE_VERSION: u32 = 2;

/// Distance sous laquelle deux valeurs sont égales (garde contre l'auto-intersection).
pub const EPSILON: f64 = 1e-4;
