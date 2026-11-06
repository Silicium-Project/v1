//! Réputation réseau des pairs (scoring gossipsub).
//!
//! Un message **rejeté** par le validateur coûte cher à son émetteur (pénalité P4) ; le
//! maillage n'est pas jugé sur sa régularité de livraison (P3 désactivé : un pair honnête
//! peut être silencieux). Au-delà des seuils, le pair n'est plus relayé puis ignoré.

use crate::Topic;
use libp2p::gossipsub::{PeerScoreParams, PeerScoreThresholds, TopicScoreParams};
use std::time::Duration;

fn topic_params() -> TopicScoreParams {
    TopicScoreParams {
        topic_weight: 0.5,
        time_in_mesh_weight: 0.01,
        time_in_mesh_quantum: Duration::from_secs(1),
        time_in_mesh_cap: 3600.0,
        first_message_deliveries_weight: 0.5,
        first_message_deliveries_decay: 0.9,
        first_message_deliveries_cap: 100.0,
        mesh_message_deliveries_weight: 0.0,
        mesh_message_deliveries_decay: 0.9,
        mesh_message_deliveries_cap: 0.0,
        mesh_message_deliveries_threshold: 0.0,
        mesh_message_deliveries_window: Duration::from_millis(10),
        mesh_message_deliveries_activation: Duration::from_secs(5),
        mesh_failure_penalty_weight: 0.0,
        mesh_failure_penalty_decay: 0.9,
        invalid_message_deliveries_weight: -20.0,
        invalid_message_deliveries_decay: 0.9,
    }
}

/// Paramètres de score, un jeu par topic connu.
pub(crate) fn params() -> (PeerScoreParams, PeerScoreThresholds) {
    let mut score = PeerScoreParams {
        topic_score_cap: 20.0,
        // Plusieurs pairs derrière une même IP : suspect (Sybil bon marché).
        ip_colocation_factor_weight: -5.0,
        ip_colocation_factor_threshold: 8.0,
        behaviour_penalty_weight: -1.0,
        behaviour_penalty_threshold: 6.0,
        behaviour_penalty_decay: 0.9,
        decay_interval: Duration::from_secs(1),
        decay_to_zero: 0.01,
        retain_score: Duration::from_secs(600),
        ..PeerScoreParams::default()
    };
    for topic in Topic::all() {
        score.topics.insert(topic.ident().hash(), topic_params());
    }
    (score, PeerScoreThresholds::default())
}
