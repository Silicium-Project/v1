//! Windows : un Job Object contient le programme et ses enfants ; sa fermeture tue tout l'arbre.
//! Pas de borne mémoire, de nombre de processus ni de réseau : `win32job` n'expose pas les
//! limites dures, et les poser demanderait du code `unsafe`, interdit dans ce projet.

use std::{os::windows::io::AsRawHandle, process::Child};
use win32job::{ExtendedLimitInfo, Job};

/// Tient le Job Object vivant : sa fermeture tue l'arbre de processus.
pub struct TreeGuard {
    _job: Job,
}

/// Crée un Job qui tue l'arbre à sa fermeture, et y place le processus lancé.
pub fn guard(child: &Child) -> Result<TreeGuard, String> {
    let mut info = ExtendedLimitInfo::new();
    info.limit_kill_on_job_close();
    let job = Job::create_with_limit_info(&info).map_err(|e| format!("job object : {e}"))?;
    job.assign_process(child.as_raw_handle() as isize)
        .map_err(|e| format!("rattachement au job object : {e}"))?;
    Ok(TreeGuard { _job: job })
}
