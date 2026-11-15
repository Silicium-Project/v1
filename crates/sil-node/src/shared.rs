use crate::options::Roles;
use sil_content::ContentStore;
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_ledger::Ledger;
use sil_ledger_store::EventStore;
use sil_net::NetworkHandle;
use sil_reputation::Reputation;
use sil_settlement::{Chain, IntentId, Outbox, OutboxFile};
use sil_types::JobId;
use sil_workload_api::Registry;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex, MutexGuard, OnceLock,
        atomic::{AtomicU32, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

/// Où en est la synchronisation d'un job auprès d'un pair.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SyncState {
    /// Position suivante dans le journal du pair.
    pub(crate) cursor: u64,
    /// Instant où le pair n'avait plus rien de nouveau à nous donner.
    pub(crate) caught_up_at: Option<u64>,
}

/// Ce que ce nœud a déjà fait, pour ne jamais le refaire (une clé = job, fragment, tentative).
#[derive(Default)]
pub(crate) struct WorkBook {
    pub(crate) claimed: BTreeSet<(JobId, u32, u32)>,
    pub(crate) running: BTreeSet<(JobId, u32, u32)>,
    pub(crate) offered: BTreeSet<(JobId, u32, u32)>,
    pub(crate) verifying: BTreeSet<(JobId, u32, u32)>,
    /// Vérifications en cours d'exécution (le rendu tourne), au plus `VERIFY_PARALLEL`.
    pub(crate) verify_running: BTreeSet<(JobId, u32, u32)>,
    /// Instant où ce nœud peut publier le Finalize d'une tentative (job, fragment, tentative).
    /// Le premier rang publie tout de suite ; les suivants attendent leur tour (voir finalize_loop).
    pub(crate) finalize_ready: BTreeMap<(JobId, u32, u32), u64>,
    /// Tentatives déjà comptées dans la réputation (job, fragment, tentative).
    pub(crate) observed: BTreeSet<(JobId, u32, u32)>,
    /// Verdicts déjà comptés dans la réputation de vérificateur (job, fragment, tentative, pair).
    pub(crate) observed_verdicts: BTreeSet<(JobId, u32, u32, sil_types::PublicKey)>,
    /// Résultat de la garde d'escrow par job : (jusqu'à quand, accepté).
    /// Position de synchronisation par (job, pair).
    pub(crate) sync: BTreeMap<(JobId, sil_net::PeerId), SyncState>,
    /// Dernière présence reçue de chaque pair, et quand.
    pub(crate) presence: BTreeMap<sil_net::PeerId, (sil_net::Presence, u64)>,
    /// Fragments de topics de jobs auxquels ce nœud est abonné.
    pub(crate) subscribed: BTreeSet<u8>,
    pub(crate) escrow_ok: BTreeMap<JobId, (u64, bool)>,
    /// Escrow lu sur la chaîne pour les gains : (jusqu'à quand, escrow). Évite un RPC par rafraîchissement.
    pub(crate) escrow_seen: BTreeMap<JobId, (u64, Option<sil_chain_core::Escrow>)>,
    /// Un verrou par contenu en cours de téléchargement : les demandeurs suivants attendent.
    pub(crate) fetching: BTreeMap<sil_types::ContentId, Arc<tokio::sync::Mutex<()>>>,
    /// Tentatives vues par ce nœud (job, fragment, tentative) : heure locale du claim et du résultat.
    /// Mesure la vitesse des pairs sans faire confiance aux horodatages déclarés.
    pub(crate) attempt_seen: BTreeMap<(JobId, u32, u32), AttemptSeen>,
    /// Premier instant où ce nœud a vu un fragment sans preneur (job, fragment).
    pub(crate) waiting_since: BTreeMap<(JobId, u32), u64>,
    /// Dernier journal de refus de claim par (job, fragment, raison) : limite le bruit des logs.
    pub(crate) refusals: BTreeMap<(JobId, u32, &'static str), u64>,
}

/// Une tentative vue par ce nœud. `claim_ms` est absent si le résultat était déjà là à la première
/// observation : la durée n'est alors pas mesurable.
#[derive(Clone)]
pub(crate) struct AttemptSeen {
    pub(crate) claimant: sil_types::PublicKey,
    pub(crate) claim_ms: Option<u64>,
    pub(crate) result_ms: Option<u64>,
}

/// État partagé du nœud. Les verrous sont tenus brièvement et jamais à travers un `await`.
pub struct Shared {
    pub(crate) ledger: Mutex<Ledger>,
    pub(crate) log: Mutex<EventStore>,
    pub(crate) store: ContentStore,
    pub(crate) registry: Registry,
    pub(crate) reputation: Mutex<Reputation>,
    /// Réputation de vérificateur : fondée sur les verdicts, elle classe les offres de vérification.
    pub(crate) verify_reputation: Mutex<Reputation>,
    /// Réputation relue dans le journal (ADR 0005, phase 1b), gardée tant que le journal ne change pas.
    pub(crate) derived: Mutex<crate::derived_reputation::Cache>,
    pub(crate) outbox: Mutex<Outbox>,
    pub(crate) outbox_file: OutboxFile,
    /// Pénalités à envoyer (ADR 0012). Reconstruites depuis le journal : pas de sauvegarde.
    pub(crate) penalties: Mutex<sil_settlement::PenaltyBox>,
    pub(crate) chain: Arc<dyn Chain>,
    pub(crate) seed: MasterSeed,
    pub(crate) identity: SigningIdentity,
    pub(crate) roles: Roles,
    /// Mode de test : pas de vérification d'escrow avant de réclamer (voir `NodeOptions`).
    pub(crate) free_work: bool,
    pub(crate) net: OnceLock<NetworkHandle>,
    pub(crate) work: Mutex<WorkBook>,
    pub(crate) runtime: tokio::runtime::Handle,
    /// Tâches de calcul ou de vérification en cours (charge annoncée).
    pub(crate) active: AtomicU32,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

impl Shared {
    pub fn now_ms(&self) -> u64 {
        wall_ms()
    }

    /// Clé de signature du journal de ce nœud.
    pub fn author(&self) -> sil_types::PublicKey {
        self.identity.public()
    }

    pub fn store(&self) -> &ContentStore {
        &self.store
    }

    pub fn net(&self) -> Option<&NetworkHandle> {
        self.net.get()
    }

    /// Nouvelle identité de signature (copie) pour un automate qui en est propriétaire.
    pub(crate) fn fresh_identity(&self) -> SigningIdentity {
        self.seed.derive(Purpose::Ledger)
    }

    /// État dérivé d'un job (pour les tests et l'API).
    pub fn job_state(&self, job: &JobId) -> Option<sil_ledger::JobState> {
        lock(&self.ledger).job_state(job, self.now_ms())
    }

    pub fn job_ids(&self) -> Vec<JobId> {
        lock(&self.ledger).job_ids()
    }

    /// S'abonne au fragment de topic d'un job connu (une seule fois par fragment).
    pub(crate) fn follow_job(&self, job: &JobId) {
        let topic = sil_net::Topic::for_job(job);
        let sil_net::Topic::Jobs(shard) = topic else {
            return;
        };
        if !lock(&self.work).subscribed.insert(shard) {
            return;
        }
        if let Some(net) = self.net.get().cloned() {
            self.runtime.spawn(async move {
                if let Err(error) = net.subscribe(topic).await {
                    tracing::warn!(%error, "abonnement au fragment de jobs impossible");
                }
            });
        }
    }

    /// État des règlements.
    pub fn settlement_report(&self) -> Vec<(IntentId, sil_settlement::Status)> {
        lock(&self.outbox).report(self.now_ms())
    }
}

/// Compte une tâche en cours pour la charge annoncée (décrémenté à la fin, même en cas d'erreur).
pub(crate) struct Busy(Arc<Shared>);

impl Busy {
    pub(crate) fn enter(shared: &Arc<Shared>) -> Self {
        shared.active.fetch_add(1, Ordering::Relaxed);
        Self(Arc::clone(shared))
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::Relaxed);
    }
}
