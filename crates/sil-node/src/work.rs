//! Exécution d'un fragment attribué à ce nœud.

use crate::{
    blob::ResultBlob,
    fetch::ensure_content,
    input::StoreInput,
    shared::{Shared, lock},
};
use sil_ledger::EventBody;
use sil_types::{ContentId, Hash32, JobId};
use sil_workload_api::{Output, Workload, WorkloadError};
use std::{sync::Arc, time::Duration};

/// Période de renouvellement : un tiers du bail, au plus 10 renouvellements (le journal n'en
/// compte pas davantage).
const MAX_RENEWALS: u32 = 10;

pub(crate) fn spawn_execution(shared: Arc<Shared>, job: JobId, fragment: u32, attempt: u32) {
    let runtime = shared.runtime.clone();
    runtime.spawn(async move {
        let lease_ms = lock(&shared.ledger)
            .spec(&job)
            .map_or(60_000, |signed| signed.spec.lease_ms);
        // Tant que le calcul tourne, on prolonge le bail : sinon un autre worker pourrait réclamer le fragment
        // worker pour un travail déjà en cours.
        let _busy = crate::shared::Busy::enter(&shared);
        let renewer = tokio::spawn(renew_lease(
            Arc::clone(&shared),
            job,
            fragment,
            attempt,
            lease_ms,
        ));
        let outcome = execute(&shared, &job, fragment).await;
        renewer.abort();
        let body = match outcome {
            Ok((result, size, milestones_root)) => EventBody::Submit {
                fragment,
                attempt,
                result,
                size,
                milestones_root,
            },
            Err(reason) => {
                tracing::warn!(%reason, fragment, attempt, "fragment abandonné");
                EventBody::Release { fragment, attempt }
            }
        };
        let _ = shared.author_event(&job, body);
    });
}

async fn renew_lease(shared: Arc<Shared>, job: JobId, fragment: u32, attempt: u32, lease_ms: u64) {
    let period = Duration::from_millis((lease_ms / 3).max(200));
    for _ in 0..MAX_RENEWALS {
        tokio::time::sleep(period).await;
        let _ = shared.author_event(&job, EventBody::Renew { fragment, attempt });
    }
}

/// Calcule le fragment et range son résultat dans le magasin.
async fn execute(
    shared: &Arc<Shared>,
    job: &JobId,
    fragment: u32,
) -> Result<(Hash32, u64, Hash32), String> {
    let spec = lock(&shared.ledger)
        .spec(job)
        .map(|s| s.spec.clone())
        .ok_or("job inconnu")?;
    let workload = shared
        .registry
        .get(&spec.workload)
        .ok_or("workload inconnu")?;
    if !ensure_content(shared, &ContentId::from_bytes(*spec.input.as_bytes())).await {
        return Err("entrée indisponible".into());
    }
    let limit = Duration::from_millis(workload.limits().wall_ms);
    let (store, input_id, fragments, shape) = (
        shared.store.clone(),
        ContentId::from_bytes(*spec.input.as_bytes()),
        spec.fragments,
        (spec.width, spec.height),
    );
    let compute = tokio::task::spawn_blocking(move || {
        compute(&*workload, &store, &input_id, fragments, fragment, shape)
    });
    let output = tokio::time::timeout(limit, compute)
        .await
        .map_err(|_| "délai dépassé")?
        .map_err(|_| "calcul interrompu")?
        .map_err(|e| e.to_string())?;

    let blob = ResultBlob::from_output(&output)
        .to_bytes()
        .map_err(|e| e.to_string())?;
    let id = shared.store.put_bytes(&blob).map_err(|e| e.to_string())?;
    shared.store.pin(&id);
    if let Some(net) = shared.net.get() {
        let _ = net.provide(id).await;
    }
    Ok((
        Hash32::from_bytes(*id.as_bytes()),
        blob.len() as u64,
        output.milestones_root(),
    ))
}

fn compute(
    workload: &dyn Workload,
    store: &sil_content::ContentStore,
    input: &ContentId,
    fragments: u32,
    fragment: u32,
    shape: (u32, u32),
) -> Result<Output, WorkloadError> {
    let reader = StoreInput::open(store.clone(), *input)?;
    // Une spec qui ne décrit pas l'entrée rendrait les montants faux : on ne calcule pas.
    workload.check_shape(&reader, shape.0, shape.1)?;
    let specs = workload.split(&reader, fragments)?;
    let spec = specs
        .get(fragment as usize)
        .ok_or_else(|| WorkloadError::BadSpec("fragment absent du découpage".into()))?;
    workload.execute(&reader, spec)
}
