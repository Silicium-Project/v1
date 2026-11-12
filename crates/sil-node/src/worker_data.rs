//! Données d'un worker : ce que ce nœud fait, lu dans sa propre chaîne d'événements, avec sa
//! réputation, sa capacité et les règlements qu'il a vus. Lecture seule : rien n'est calculé sur
//! la chaîne.

use crate::shared::{Shared, lock};
use crate::worker_tasks::{self, HISTORY};
use serde_json::{Value, json};
use sil_chain_core::Escrow;
use sil_ledger::EventBody;
use sil_settlement::terms_of;
use sil_types::JobId;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};
use sysinfo::{Pid, ProcessesToUpdate, System};

/// Nombre d'événements récents renvoyés (le reste est dans les compteurs).
const RECENT: usize = 50;

/// Échantillonneur CPU : sysinfo calcule l'usage entre deux rafraîchissements, il vit donc avec le nœud.
fn cpu_sampler() -> &'static Mutex<System> {
    static SAMPLER: OnceLock<Mutex<System>> = OnceLock::new();
    SAMPLER.get_or_init(|| Mutex::new(System::new()))
}

/// Charge du processus `silicium-node` (en % de l'ensemble des cœurs) et du PC entier, en %.
/// Le premier appel n'a pas encore d'intervalle : il renvoie des valeurs proches de zéro.
fn cpu_load(cores: usize) -> (f64, f64) {
    let mut system = lock(cpu_sampler());
    let pid = Pid::from_u32(std::process::id());
    system.refresh_cpu_usage();
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let machine = f64::from(system.global_cpu_usage());
    // sysinfo compte 100 % pour un cœur entier : on ramène à l'ensemble des cœurs.
    let node = system
        .process(pid)
        .map_or(0.0, |p| f64::from(p.cpu_usage()))
        / cores.max(1) as f64;
    (round1(node), round1(machine))
}

/// Un chiffre après la virgule, pour un affichage stable.
fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

/// Nom stable d'un type d'événement, tel que l'API et l'interface le montrent.
pub fn kind_of(body: &EventBody) -> &'static str {
    match body {
        EventBody::JobPublished { .. } => "job_published",
        EventBody::Claim { .. } => "claim",
        EventBody::Renew { .. } => "renew",
        EventBody::Release { .. } => "release",
        EventBody::Submit { .. } => "submit",
        EventBody::VerifyOffer { .. } => "verify_offer",
        EventBody::Attest { .. } => "attest",
        EventBody::AttestTolerance { .. } => "attest_tolerance",
        EventBody::Finalize { .. } => "finalize",
        EventBody::FinalizeTolerance { .. } => "finalize_tolerance",
        EventBody::Cancel => "cancel",
    }
}

/// Rapport du nœud en tant que worker. `payments` reste faux tant que l'adaptateur de chaîne
/// n'existe pas : aucun gain n'est donc affiché comme acquis.
pub fn report(shared: &Shared) -> Value {
    let me = shared.author();
    let now = shared.now_ms();
    let mut counts: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut recent: Vec<(u64, Value)> = Vec::new();
    let mut running: Vec<Value> = Vec::new();
    let mut history: Vec<(u64, Value)> = Vec::new();
    let (mut jobs, mut finalized, mut other) = (0u64, 0u64, 0u64);
    {
        let ledger = lock(&shared.ledger);
        for job in ledger.job_ids() {
            let events = ledger.events_of(&job, &me);
            if events.is_empty() {
                continue;
            }
            jobs += 1;
            for event in events {
                let kind = kind_of(&event.body);
                *counts.entry(kind).or_insert(0) += 1;
                recent.push((
                    event.issued_at_ms,
                    json!({
                        "job": job.to_hex(),
                        "sequence": event.sequence,
                        "kind": kind,
                        "issued_at_ms": event.issued_at_ms,
                    }),
                ));
            }
            if let Some(state) = ledger.job_state(&job, now) {
                let parts = worker_tasks::of_job(&state, &me);
                running.extend(parts.running);
                history.extend(parts.history);
                finalized += parts.finalized;
                other += parts.other;
            }
        }
    }
    recent.sort_by_key(|event| std::cmp::Reverse(event.0));
    recent.truncate(RECENT);
    history.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    history.truncate(HISTORY);

    let (reputation, (successes, failures)) = {
        let book = lock(&shared.reputation);
        (book.score(&me, now), book.evidence(&me, now))
    };
    let max_parallel = crate::worker_loop::max_parallel(shared, &me, now);
    // Réputation relue dans le journal (ADR 0005, phase 1b) : affichée, pas encore décisive.
    let trusted = lock(&shared.reputation).trusted_scopes();
    let (derived_score, (derived_good, derived_bad), derived_at) = {
        let ledger = lock(&shared.ledger);
        let mut cache = lock(&shared.derived);
        let derived = cache.get(&ledger, &trusted);
        (
            derived.score(&me),
            derived.evidence(&me),
            derived.as_of_ms(),
        )
    };
    // Réputation réseau, lue on-chain (ADR 0006) : `null` tant que le compte n'existe pas.
    let standing_onchain = shared.chain.verifier_standing(&me).ok().flatten();
    let cores = std::thread::available_parallelism().map_or(1, NonZeroUsize::get);
    let (node_cpu_pct, machine_cpu_pct) = cpu_load(cores);
    let mut settlements: BTreeMap<String, u64> = BTreeMap::new();
    for (_, status) in shared.settlement_report() {
        *settlements.entry(format!("{status:?}")).or_insert(0) += 1;
    }

    json!({
        "worker": me.to_hex(),
        "roles": {
            "compute": shared.roles.worker,
            "verify": shared.roles.verifier,
        },
        "jobs_touched": jobs,
        "events_by_kind": counts,
        "recent": recent.into_iter().map(|(_, event)| event).collect::<Vec<_>>(),
        "reputation": reputation,
        "standing_onchain": standing_onchain,
        "reputation_evidence": { "successes": successes, "failures": failures },
        "reputation_derived": {
            "score": derived_score,
            "evidence": { "successes": derived_good, "failures": derived_bad },
            "as_of_ms": derived_at,
        },
        "capacity": {
            "active": shared.active.load(Ordering::Relaxed),
            "max_parallel": max_parallel,
            "cpu_cores": cores,
            "node_cpu_pct": node_cpu_pct,
            "machine_cpu_pct": machine_cpu_pct,
        },
        "in_progress": running,
        "history": history.into_iter().map(|(_, entry)| entry).collect::<Vec<_>>(),
        "history_totals": { "finalized": finalized, "other": other },
        "settlements_seen": settlements,
        "earnings": earnings(shared, &me, now),
        "payments": false,
    })
}

/// Durée de validité d'un escrow lu pour les gains.
const ESCROW_SEEN_MS: u64 = 30_000;

/// Gains de ce noeud, confirmes sur la chaine : un fragment compte une fois regle on-chain.
/// Worker : unites x tarif par unite. Verificateur : unites x tarif de verification, pour chaque
/// verificateur qui a approuve le fragment. Rien n'est compte avant le reglement.
fn earnings(shared: &Shared, me: &sil_types::PublicKey, now: u64) -> Value {
    // Passe 1 (journal) : les fragments finalises ou je suis worker ou approbateur.
    type Fragments = Vec<(u32, bool, bool)>;
    let mut entries: BTreeMap<JobId, (sil_chain_core::EscrowTerms, Fragments)> = BTreeMap::new();
    {
        let ledger = lock(&shared.ledger);
        for job in ledger.job_ids() {
            let Some(signed) = ledger.spec(&job) else {
                continue;
            };
            let terms = terms_of(&signed.spec);
            for fragment in 0..signed.spec.fragments {
                let Some(state) = ledger.task_state(&job, fragment, now) else {
                    continue;
                };
                let Some(done) = state.finalized else {
                    continue;
                };
                let as_worker = done.claimant == *me;
                let as_approver = done.approvers.contains(me);
                if as_worker || as_approver {
                    entries
                        .entry(job)
                        .or_insert_with(|| (terms.clone(), Vec::new()))
                        .1
                        .push((fragment, as_worker, as_approver));
                }
            }
        }
    }
    // Passe 2 (chaine) : seuls les fragments reglés on-chain comptent.
    let (mut worker, mut verifier, mut settled) = (0u64, 0u64, 0u64);
    for (job, (terms, fragments)) in entries {
        let Some(escrow) = settled_escrow(shared, &job, now) else {
            continue;
        };
        for (fragment, as_worker, as_approver) in fragments {
            if escrow.settled(fragment).is_none() {
                continue;
            }
            let units = terms.fragment_units(fragment).unwrap_or(0);
            if as_worker {
                worker += units.saturating_mul(terms.reward_per_unit);
            }
            if as_approver {
                verifier += units.saturating_mul(terms.verifier_reward_per_unit);
            }
            settled += 1;
        }
    }
    json!({
        "worker_lamports": worker,
        "verifier_lamports": verifier,
        "lamports": worker.saturating_add(verifier),
        "fragments_settled": settled,
        "checked_at_ms": now,
    })
}

/// Escrow d'un job lu sur la chaine, avec un cache court (un RPC par job et par intervalle).
fn settled_escrow(shared: &Shared, job: &JobId, now: u64) -> Option<Escrow> {
    if let Some((until, escrow)) = lock(&shared.work).escrow_seen.get(job).cloned()
        && now < until
    {
        return escrow;
    }
    let escrow = shared.chain.escrow(job).ok().flatten();
    lock(&shared.work)
        .escrow_seen
        .insert(*job, (now + ESCROW_SEEN_MS, escrow.clone()));
    escrow
}
