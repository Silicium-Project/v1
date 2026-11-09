use sil_net::{Multiaddr, NetLimits};
use sil_quota::LedgerLimits;
use std::{net::SocketAddr, path::PathBuf};

/// Ce que ce nœud fait pour le réseau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Roles {
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
    /// Relais public : ce nœud doit être joignable directement.
    pub relay_server: bool,
    pub control: Option<ControlOptions>,
    pub ledger_limits: LedgerLimits,
    pub net_limits: NetLimits,
    /// Requesters dont ce nœud croit les journaux pour sa réputation (voir `sil-reputation`).
    pub trusted_requesters: Vec<sil_types::PublicKey>,
    /// Période des boucles de décision.
    pub tick_ms: u64,
    /// Mode de test : le worker calcule sans escrow vérifié. Jamais avec une chaîne (voir `sil-config`).
    pub free_work: bool,
}

impl NodeOptions {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            roles: Roles::default(),
            listen: Vec::new(),
            bootstrap: Vec::new(),
            relay_server: false,
            control: None,
            ledger_limits: LedgerLimits::default(),
            net_limits: NetLimits::default(),
            trusted_requesters: Vec::new(),
            tick_ms: 500,
            free_work: false,
        }
    }
}
