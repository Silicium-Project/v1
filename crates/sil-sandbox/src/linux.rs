//! Isolation noyau sous Linux : limites de ressources (rlimit), système de fichiers (Landlock),
//! réseau (seccomp). Appelée par le lanceur `sil-sandbox-launch`, dans le processus qui devient
//! ensuite le programme : tout ce qui est appliqué ici est hérité par le programme.

use crate::Isolation;
use landlock::{
    ABI, Access, AccessFs, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreatedAttr,
};
use rlimit::Resource;
use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
    SeccompRule, TargetArch,
};
use std::{collections::BTreeMap, convert::TryInto, path::Path};

/// Applique la politique au processus courant. Une protection qui manque fait échouer tout le
/// lancement : jamais d'exécution partiellement isolée.
pub fn apply(policy: &Isolation) -> Result<(), String> {
    limits(policy)?;
    landlock(policy)?;
    if !policy.network {
        no_network()?;
    }
    Ok(())
}

fn limits(policy: &Isolation) -> Result<(), String> {
    Resource::AS
        .set(policy.memory_bytes, policy.memory_bytes)
        .map_err(|e| format!("limite mémoire : {e}"))?;
    Resource::FSIZE
        .set(policy.max_file_bytes, policy.max_file_bytes)
        .map_err(|e| format!("limite de fichier : {e}"))?;
    Resource::CPU
        .set(policy.cpu_seconds, policy.cpu_seconds)
        .map_err(|e| format!("limite CPU : {e}"))?;
    Ok(())
}

/// Ouvre un chemin pour une règle Landlock ; un chemin absent fait échouer le lancement.
fn open(path: &Path) -> Result<PathFd, String> {
    PathFd::new(path).map_err(|e| format!("{} : {e}", path.display()))
}

/// Lecture et exécution des chemins système et du programme ; lecture-écriture seulement pour le
/// dossier de travail, sans exécution. Tout le reste du système de fichiers est refusé.
/// Les droits suivent le type de chemin : Landlock refuse les droits de dossier sur un fichier.
fn landlock(policy: &Isolation) -> Result<(), String> {
    let abi = ABI::V1;
    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| format!("landlock : {e}"))?
        .create()
        .map_err(|e| format!("landlock indisponible sur ce noyau : {e}"))?;
    for path in &policy.read_only {
        let rights = if path.is_dir() {
            AccessFs::from_read(abi) | AccessFs::Execute
        } else {
            AccessFs::ReadFile | AccessFs::Execute
        };
        ruleset = ruleset
            .add_rule(PathBeneath::new(open(path)?, rights))
            .map_err(|e| format!("landlock : {e}"))?;
    }
    for path in &policy.writable {
        let rights = if path.is_dir() {
            AccessFs::from_all(abi) ^ AccessFs::Execute
        } else {
            AccessFs::ReadFile | AccessFs::WriteFile
        };
        ruleset = ruleset
            .add_rule(PathBeneath::new(open(path)?, rights))
            .map_err(|e| format!("landlock : {e}"))?;
    }
    ruleset
        .restrict_self()
        .map_err(|e| format!("landlock : {e}"))?;
    Ok(())
}

/// Refuse la création de sockets réseau (IPv4, IPv6, netlink, paquets bruts) : `socket` renvoie
/// EPERM. Les autres appels système ne sont pas touchés.
fn no_network() -> Result<(), String> {
    let arch: TargetArch = std::env::consts::ARCH
        .try_into()
        .map_err(|e| format!("architecture : {e}"))?;
    let mut rules = Vec::new();
    for family in [
        libc::AF_INET,
        libc::AF_INET6,
        libc::AF_NETLINK,
        libc::AF_PACKET,
    ] {
        let condition =
            SeccompCondition::new(0, SeccompCmpArgLen::Dword, SeccompCmpOp::Eq, family as u64)
                .map_err(|e| format!("seccomp : {e}"))?;
        rules.push(SeccompRule::new(vec![condition]).map_err(|e| format!("seccomp : {e}"))?);
    }
    let mut map = BTreeMap::new();
    map.insert(libc::SYS_socket, rules);
    let filter = SeccompFilter::new(
        map,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        arch,
    )
    .map_err(|e| format!("seccomp : {e}"))?;
    let program: BpfProgram = filter.try_into().map_err(|e| format!("seccomp : {e}"))?;
    seccompiler::apply_filter(&program).map_err(|e| format!("seccomp indisponible : {e}"))?;
    Ok(())
}
