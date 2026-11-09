//! Assemblage de l'état d'une tâche et de celui du job.

use super::{
    attempt, finalize,
    index::View,
    types::{AttemptState, JobState, JobStatus, TaskState, TaskStatus},
};
use crate::{Event, spec::JobSpec};
use sil_types::{Hash32, JobId, PublicKey, TaskRef};
use std::collections::{BTreeMap, BTreeSet};

/// Statut d'une tentative : un résultat l'emporte sur un bail expiré ou un abandon.
fn attempt_status(spec: &JobSpec, state: &AttemptState, now_ms: u64) -> TaskStatus {
    if state.result.is_some() {
        let needed = usize::try_from(spec.verifications_required).unwrap_or(usize::MAX);
        let approved = state.approvals() >= needed;
        let rejected = state.rejections() >= needed;
        return match (approved, rejected) {
            (true, true) => TaskStatus::Disputed,
            (true, false) => TaskStatus::Verified,
            (false, true) => TaskStatus::Rejected,
            (false, false) => TaskStatus::Completed,
        };
    }
    if state.released {
        TaskStatus::Released
    } else if now_ms > state.lease_expires_at_ms {
        TaskStatus::LeaseExpired
    } else {
        TaskStatus::Claimed
    }
}

fn status_of(
    spec: &JobSpec,
    attempts: &[AttemptState],
    cancelled: bool,
    now_ms: u64,
) -> (TaskStatus, Option<u32>) {
    if cancelled {
        return (TaskStatus::Cancelled, None);
    }
    let expired = now_ms > spec.expires_at_ms;
    let Some(last) = attempts.last() else {
        return if expired {
            (TaskStatus::Expired, None)
        } else {
            (TaskStatus::Open, Some(1))
        };
    };
    if expired {
        return (TaskStatus::Expired, None);
    }
    let status = attempt_status(spec, last, now_ms);
    if !status.is_failed_attempt() {
        return (status, None);
    }
    if last.attempt < spec.max_attempts {
        (status, Some(last.attempt + 1))
    } else {
        (TaskStatus::Failed, None)
    }
}

pub(super) fn derive_task(view: &View<'_>, fragment: u32, now_ms: u64) -> TaskState {
    let events = view.task_events(fragment);
    let mut attempts = Vec::new();
    for k in 1..=view.spec.max_attempts {
        let Some(won) = attempt::accepted(view, events, k, &attempts) else {
            break;
        };
        let state = attempt::detail(view, events, k, &won);
        attempts.push(state);
    }
    let finalized = finalize::find(view, events);
    let (status, next_attempt) = match &finalized {
        Some(_) => (TaskStatus::Finalized, None),
        None => status_of(view.spec, &attempts, view.cancelled, now_ms),
    };
    TaskState {
        task: TaskRef::new(view.job, fragment),
        status,
        attempts,
        next_attempt,
        finalized,
    }
}

/// État complet du job à l'instant `now_ms`.
pub(crate) fn derive_job(
    job: JobId,
    spec: &JobSpec,
    events: &BTreeMap<Hash32, Event>,
    equivocators: &BTreeSet<PublicKey>,
    now_ms: u64,
) -> JobState {
    let view = View::new(job, spec, events, equivocators);
    let tasks: Vec<TaskState> = (0..spec.fragments)
        .map(|fragment| derive_task(&view, fragment, now_ms))
        .collect();
    let status = if tasks.iter().all(|t| t.status == TaskStatus::Finalized) {
        JobStatus::Finalized
    } else if view.cancelled {
        JobStatus::Cancelled
    } else if now_ms > spec.expires_at_ms {
        JobStatus::Expired
    } else {
        JobStatus::Active
    };
    JobState { job, status, tasks }
}
