use crate::JobId;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Une tâche = un fragment d'un job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskRef {
    pub job: JobId,
    pub fragment: u32,
}

impl TaskRef {
    pub const fn new(job: JobId, fragment: u32) -> Self {
        Self { job, fragment }
    }
}

impl fmt::Display for TaskRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.job, self.fragment)
    }
}
