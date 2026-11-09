//! Ce qu'il faut pour relire le temps dans le journal : sa version et l'heure de tête par auteur.

use super::Ledger;
use sil_types::{Hash32, JobId, PublicKey};

impl Ledger {
    /// Version du journal : croît à chaque événement accepté, reste fixe sinon.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Séquence et identifiant du dernier événement accepté de `author` dans `job` : ce qu'il faut
    /// pour émettre l'événement suivant de sa chaîne (`sequence + 1`, `previous = identifiant`).
    pub fn head(&self, job: &JobId, author: &PublicKey) -> Option<(u64, Hash32)> {
        self.jobs.get(job)?.chains.head(author)
    }

    /// Heure d'émission du dernier événement accepté de `author` dans `job`, s'il existe.
    pub fn head_time(&self, job: &JobId, author: &PublicKey) -> Option<u64> {
        self.jobs.get(job)?.head_time(author)
    }
}
