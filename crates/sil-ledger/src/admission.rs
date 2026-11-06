use crate::SignedJobSpec;
use sil_types::JobId;

/// Politique d'admission d'un nouveau job, injectée par le nœud.
///
/// C'est le point d'accroche de la protection anti-spam économique : en phase 5, une
/// implémentation vérifiera qu'un escrow existe pour `job`. Sans elle, seuls les quotas
/// de `sil-quota` limitent la création de jobs.
pub trait Admission: Send + Sync {
    fn admit(&self, spec: &SignedJobSpec, job: &JobId) -> bool;
}

/// Accepte tout (tests, réseaux locaux).
#[derive(Debug, Clone, Copy, Default)]
pub struct AllowAll;

impl Admission for AllowAll {
    fn admit(&self, _spec: &SignedJobSpec, _job: &JobId) -> bool {
        true
    }
}
