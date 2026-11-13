//! Primitives bayésiennes déterministes (ADR 0013) : même calcul, même résultat sur tout nœud.
//!
//! - [`BetaPosterior`] : taux (succès, fausse vérification, rétention après audit).
//! - [`NormalGammaPosterior`] : mesure continue (débit, latence, vitesse de calcul).
//! - [`fixed`] : les primitives numériques (`exp`, `ln`, `√`, `Φ`) qui les rendent possibles sans
//!   flottant système, exposées pour les modules qui en ont besoin directement (diagrammes,
//!   seuils dynamiques — ADR 0013 §5, §7).
//!
//! Rien ici ne touche à l'argent : ces scores servent de *prior* à une décision d'attribution
//! (ADR 0013 §6), jamais à un règlement ou une pénalité (ADR 0006, 0008, 0010, 0012).

pub mod fixed;

mod beta;
mod gauss;

pub use beta::BetaPosterior;
pub use fixed::MICRO;
pub use gauss::NormalGammaPosterior;
