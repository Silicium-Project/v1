//! API HTTP de contrôle d'un nœud (docs/technical/security/control-api.md).
//!
//! Corrige les défauts de l'API de V0 :
//! - jeton comparé avec `!=` → comparaison **en temps constant** de tous les jetons, **scopes**
//!   (`read`, `submit`), aucun accès anonyme hors `/health` et la description OpenAPI ;
//! - chemin de fichier absolu accepté (lecture arbitraire du disque du nœud, puis diffusion
//!   par hash) → **aucun chemin** n'est accepté : le contenu est envoyé **en octets** ;
//! - aucune limite → limite de **débit** par jeton, **taille** bornée des corps, protection
//!   contre l'essai de jetons à répétition ;
//! - description de l'API écrite à la main → **OpenAPI généré** de la même table que le routeur.
//!
//! Le crate ne connaît pas le nœud : il s'appuie sur le trait [`Backend`].

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod auth;
mod backend;
mod error;
mod guard;
mod handlers;
mod openapi;
mod router;
mod routes;
mod server;

pub use auth::{Principal, Scope, TokenError, TokenSet};
pub use backend::{Backend, EventReceipt};
pub use error::ApiError;
pub use router::{ApiLimits, router};
pub use routes::{ROUTES, Route};
pub use server::serve;

#[cfg(test)]
mod tests;
