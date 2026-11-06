//! Vérificateur : propose de vérifier, puis recalcule un échantillon et publie son verdict.

use crate::{
    blob::ResultBlob,
    fetch::ensure_content,
    input::{StoreInput, read_all},
    shared::{Shared, lock},
};
use sil_attest::{Attestation, AttestationBody};
use sil_content::ContentStore;
use sil_crypto::SigningIdentity;
use sil_ledger::{EventBody, TaskState, TaskStatus};
use sil_sampling::{SamplingSeed, sample_indices, samples_needed};
use sil_types::{ContentId, Hash32, JobId, TaskRef};
use sil_workload_api::{Evidence, Output, Verdict, VerificationPolicy, Workload};
use std::{sync::Arc, time::Duration};
use tokio::{sync::watch, time::interval};

/// Taille maximale d'un résultat lu pour vérification.
const MAX_BLOB_BYTES: usize = 256 * 1024 * 1024;
/// Délai avant de retenter une vérification qui n'a pas pu s'exécuter.
const RETRY_AFTER: Duration = Duration::from_secs(5);

pub(crate) async fn run(shared: Arc<Shared>, tick: Duration, mut stop: watch::Receiver<bool>) {
    let mut timer = interval(tick);
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = timer.tick() => {
                for job in shared.job_ids() {
                    scan_job(&shared, &job);
                }
            }
        }
    }
}

fn awaiting_verification(state: &TaskState) -> bool {
    matches!(state.status, TaskStatus::Completed | TaskStatus::Disputed)
}

fn scan_job(shared: &Arc<Shared>, job: &JobId) {
    let now = shared.now_ms();
    let (workload_known, fragments) = {
        let ledger = lock(&shared.ledger);
        match ledger.spec(job) {
            Some(spec) => (
                shared.registry.get(&spec.spec.workload).is_some(),
                spec.spec.fragments,
            ),
            None => return,
        }
    };
    if !workload_known {
        return;
    }
    let me = shared.identity.public();
    for fragment in 0..fragments {
        let Some(state) = lock(&shared.ledger).task_state(job, fragment, now) else {
            continue;
        };
        let Some(attempt) = state.attempts.last() else {
            continue;
        };
        if !awaiting_verification(&state) || attempt.claimant == me || attempt.result.is_none() {
            continue;
        }
        let key = (*job, fragment, attempt.attempt);
        let mine = attempt.verifiers.iter().find(|v| v.verifier == me);
        match mine {
            None => {
                if lock(&shared.work).offered.insert(key) {
                    let body = EventBody::VerifyOffer {
                        fragment,
                        attempt: attempt.attempt,
                    };
                    let _ = shared.author_event(job, body);
                }
            }
            Some(v) if v.verdict.is_none() => {
                if lock(&shared.work).verifying.insert(key) {
                    spawn_verification(Arc::clone(shared), *job, fragment, attempt.attempt);
                }
            }
            Some(_) => {}
        }
    }
}

fn spawn_verification(shared: Arc<Shared>, job: JobId, fragment: u32, attempt: u32) {
    let runtime = shared.runtime.clone();
    runtime.spawn(async move {
        let _busy = crate::shared::Busy::enter(&shared);
        match verify(&shared, &job, fragment, attempt).await {
            Ok(body) => {
                let _ = shared.author_event(&job, body);
            }
            Err(reason) => {
                tracing::warn!(%reason, fragment, attempt, "vérification non menée");
                tokio::time::sleep(RETRY_AFTER).await;
                lock(&shared.work)
                    .verifying
                    .remove(&(job, fragment, attempt));
            }
        }
    });
}

/// Retourne l'événement `Attest` à publier. Une erreur veut dire « pas de verdict possible ».
async fn verify(
    shared: &Arc<Shared>,
    job: &JobId,
    fragment: u32,
    attempt: u32,
) -> Result<EventBody, String> {
    let now = shared.now_ms();
    let spec = lock(&shared.ledger)
        .spec(job)
        .map(|s| s.spec.clone())
        .ok_or("job inconnu")?;
    let workload = shared
        .registry
        .get(&spec.workload)
        .ok_or("workload inconnu")?;
    let submitted = lock(&shared.ledger)
        .task_state(job, fragment, now)
        .and_then(|s| {
            s.attempts
                .get(attempt as usize - 1)
                .and_then(|a| a.result.clone())
        })
        .ok_or("pas de résultat")?;

    let input_id = ContentId::from_bytes(*spec.input.as_bytes());
    let result_id = ContentId::from_bytes(*submitted.result.as_bytes());
    if !ensure_content(shared, &input_id).await || !ensure_content(shared, &result_id).await {
        return Err("contenu indisponible".into());
    }
    let blob = read_all(&shared.store, &result_id).map_err(|e| e.to_string())?;
    let blob = ResultBlob::from_bytes(&blob, MAX_BLOB_BYTES).map_err(|e| e.to_string())?;
    let output = blob.into_output();

    let task = TaskRef::new(*job, fragment);
    let case = Case {
        store: shared.store.clone(),
        input: input_id,
        fragments: spec.fragments,
        fragment,
        task,
        attempt,
        result: submitted.result,
        committed_root: submitted.milestones_root,
    };
    let identity = shared.fresh_identity();
    let verdict = tokio::task::spawn_blocking(move || judge(&*workload, &identity, &case, &output))
        .await
        .map_err(|_| "vérification interrompue")??;

    let (approved, evidence) = match verdict {
        Verdict::Approve => (true, None),
        Verdict::Reject(evidence) => {
            let bytes = evidence.to_bytes();
            if let Ok(id) = shared.store.put_bytes(&bytes) {
                shared.store.pin(&id);
            }
            (false, Some(evidence.hash()))
        }
    };
    let attestation = Attestation::sign(
        &shared.identity,
        AttestationBody {
            task,
            attempt,
            result: submitted.result,
            approved,
            evidence,
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(EventBody::Attest { attestation })
}

/// Indices de blocs à recalculer selon la politique du workload et la graine du vérificateur.
fn sample_for(
    policy: VerificationPolicy,
    identity: &SigningIdentity,
    case: &Case,
    blocks: u32,
) -> Vec<u32> {
    match policy {
        VerificationPolicy::Recompute => Vec::new(),
        VerificationPolicy::Sampling {
            min_cheat_fraction,
            target_detection,
        } => {
            let (seed, _) = SamplingSeed::draw(identity, &case.task, case.attempt, &case.result);
            let wanted = samples_needed(blocks, min_cheat_fraction, target_detection);
            sample_indices(&seed, blocks, wanted)
        }
    }
}

/// Tout ce qu'il faut pour juger un résultat (propriétaire de ses données : passe en thread).
struct Case {
    store: ContentStore,
    input: ContentId,
    fragments: u32,
    fragment: u32,
    task: TaskRef,
    attempt: u32,
    result: Hash32,
    committed_root: Hash32,
}

fn judge(
    workload: &dyn Workload,
    identity: &SigningIdentity,
    case: &Case,
    output: &Output,
) -> Result<Verdict, String> {
    let reader = StoreInput::open(case.store.clone(), case.input).map_err(|e| e.to_string())?;
    let specs = workload
        .split(&reader, case.fragments)
        .map_err(|e| e.to_string())?;
    let spec = specs
        .get(case.fragment as usize)
        .ok_or("fragment absent du découpage")?;
    let found = output.milestones_root();
    if found != case.committed_root {
        return Ok(Verdict::Reject(Evidence::new(
            0,
            case.committed_root,
            found,
            "racine des milestones différente de l'engagement",
        )));
    }
    let blocks = workload.block_count(spec).map_err(|e| e.to_string())?;
    let sample = sample_for(workload.policy(), identity, case, blocks);
    workload
        .verify(&reader, spec, output, &sample)
        .map_err(|e| e.to_string())
}
