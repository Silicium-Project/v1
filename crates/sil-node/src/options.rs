use sil_net::{Multiaddr, NetLimits};
use sil_quota::LedgerLimits;
use std::{net::SocketAddr, path::PathBuf};

/// Ce que ce nœud fait pour le réseau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Roles {
    /// Vote au nom du comité des jobs dont il est membre, signe les règlements.
    pub coordinator: bool,
    /// Réclame et exécute des fragments.
    pub worker: bool,
    /// Vérifie les résultats des autres.
    pub verifier: bool,
}

#[derive(Debug, Clone)]
pub struct ControlOptions {
    pub bind: SocketAddr,
    /// Fichier des jetons (créé avec un jeton aléatoire s'il n'existe pas).
    pub token_file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct NodeOptions {
    pub data_dir: PathBuf,
    pub roles: Roles,
    pub listen: Vec<Multiaddr>,
    pub bootstrap: Vec<Multiaddr>,
    pub control: Option<ControlOptions>,
    pub ledger_limits: LedgerLimits,
    pub net_limits: NetLimits,
    /// Requesters dont ce nœud croit les journaux pour sa réputation (voir `sil-reputation`).
    pub trusted_requesters: Vec<sil_types::PublicKey>,
    /// Période des boucles de décision.
    pub tick_ms: u64,
    /// Fenêtre d'observation des claims avant de voter (voir `sil-coordinator`).
    pub claim_window_ms: u64,
    /// Attente du vote du chef de tour avant de voter soi-même (voir `sil-coordinator`).
    pub leader_wait_ms: u64,
}

impl NodeOptions {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            roles: Roles::default(),
            listen: Vec::new(),
            bootstrap: Vec::new(),
            control: None,
            ledger_limits: LedgerLimits::default(),
            net_limits: NetLimits::default(),
            trusted_requesters: Vec::new(),
            tick_ms: 500,
            claim_window_ms: 1_000,
            leader_wait_ms: 3_000,
        }
    }
}
