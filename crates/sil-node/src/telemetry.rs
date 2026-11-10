//! Télémétrie de la machine : CPU, mémoire, rôles et tâches, envoyés à l'API toutes les minutes.
//!
//! Le message est signé par la clé du nœud (la clé liée au compte). Il porte un préfixe de domaine
//! (`sil/v1/telemetry`) : une signature d'un autre message ne vaut pas ici. L'API refuse tout ce qui
//! n'est pas signé par un appareil lié et non révoqué. Rien d'autre n'est envoyé : ni adresse, ni
//! identifiant de pair, ni contenu de tâche.

use crate::options::TelemetryOptions;
use crate::shared::Shared;
use crate::worker_data;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tokio::sync::watch;

/// Préfixe signé : le domaine du message, suivi d'un saut de ligne.
const CONTEXT: &str = "sil/v1/telemetry\n";
/// Chemin de l'API, ajouté à `api_url`.
const PATH: &str = "/v1/devices/telemetry";
const TIMEOUT: Duration = Duration::from_secs(10);

/// Boucle d'envoi, arrêtée avec le nœud.
pub(crate) async fn run(
    shared: Arc<Shared>,
    options: TelemetryOptions,
    mut stop: watch::Receiver<bool>,
) {
    let url = format!("{}{PATH}", options.api_url.trim_end_matches('/'));
    let identity = shared.fresh_identity();
    let device_key = identity.public().to_hex();
    let mut system = System::new();
    // Le CPU se mesure entre deux rafraîchissements : le premier est pris au démarrage.
    system.refresh_cpu_usage();
    let mut ticker = tokio::time::interval(options.interval);
    ticker.tick().await;
    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = stop.changed() => break,
        }
        system.refresh_cpu_usage();
        system.refresh_memory();
        let payload = snapshot(&shared, &system).to_string();
        let signature = identity.sign_raw(format!("{CONTEXT}{payload}").as_bytes());
        let body = json!({
            "device_key": device_key,
            "payload": payload,
            "signature": signature.to_hex(),
        });
        let target = url.clone();
        let sent = tokio::task::spawn_blocking(move || {
            ureq::post(&target)
                .timeout(TIMEOUT)
                .send_json(body)
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
        .await;
        match sent {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%error, "télémétrie non envoyée"),
            Err(error) => tracing::warn!(%error, "envoi de télémétrie interrompu"),
        }
    }
}

/// Ce que la machine dit d'elle-même à cet instant. Les compteurs viennent du rapport du nœud.
fn snapshot(shared: &Shared, system: &System) -> Value {
    let report = worker_data::report(shared);
    let capacity = &report["capacity"];
    let totals = &report["history_totals"];
    let total_memory = system.total_memory();
    let mem_pct = if total_memory == 0 {
        0.0
    } else {
        system.used_memory() as f64 * 100.0 / total_memory as f64
    };
    json!({
        "observed_at_ms": shared.now_ms(),
        "version": env!("CARGO_PKG_VERSION"),
        "cpu_pct": round(f64::from(system.global_cpu_usage())),
        "mem_pct": round(mem_pct),
        "cpu_cores": capacity["cpu_cores"],
        "roles": {
            "compute": report["roles"]["compute"],
            "verify": report["roles"]["verify"],
        },
        "tasks_active": capacity["active"],
        "tasks_in_progress": report["in_progress"].as_array().map_or(0, Vec::len),
        "tasks_finalized": totals["finalized"],
        "tasks_other": totals["other"],
    })
}

/// Un chiffre à une décimale : le message reste court et stable.
fn round(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_rounded_to_one_decimal() {
        assert_eq!(round(31.26), 31.3);
        assert_eq!(round(0.0), 0.0);
    }
}
