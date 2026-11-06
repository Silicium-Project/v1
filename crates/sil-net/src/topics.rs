use libp2p::gossipsub::IdentTopic;
use sil_types::JobId;

/// Nombre de topics de jobs. Fixe : le scoring gossipsub exige des topics connus d'avance
/// et un topic par job laisserait n'importe qui en créer sans limite.
pub const SHARDS: u8 = 16;

/// Les seuls topics du réseau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Topic {
    /// Présence minimale des pairs (rôles, charge).
    Presence,
    /// Annonce de nouveaux jobs (`JobPublished`).
    Announce,
    /// Événements des jobs dont l'identifiant tombe dans ce fragment (`0..SHARDS`).
    Jobs(u8),
}

impl Topic {
    /// Fragment qui porte les événements de `job`.
    pub fn for_job(job: &JobId) -> Self {
        Self::Jobs(job.as_bytes()[0] % SHARDS)
    }

    pub fn name(self) -> String {
        match self {
            Self::Presence => "sil/v1/presence".into(),
            Self::Announce => "sil/v1/announce".into(),
            Self::Jobs(shard) => format!("sil/v1/jobs/{shard}"),
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "sil/v1/presence" => Some(Self::Presence),
            "sil/v1/announce" => Some(Self::Announce),
            _ => {
                let shard: u8 = name.strip_prefix("sil/v1/jobs/")?.parse().ok()?;
                (shard < SHARDS && Self::Jobs(shard).name() == name).then_some(Self::Jobs(shard))
            }
        }
    }

    pub(crate) fn ident(self) -> IdentTopic {
        IdentTopic::new(self.name())
    }

    /// Tous les topics possibles.
    pub fn all() -> Vec<Self> {
        let mut topics = vec![Self::Presence, Self::Announce];
        topics.extend((0..SHARDS).map(Self::Jobs));
        topics
    }
}
