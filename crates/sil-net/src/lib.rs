//! Réseau de V1 (libp2p). Voir docs/technical/protocol/gossip.md.
//!
//! Différences avec V0 :
//! - **topics séparés** et bornés : `presence` (message minimal), `announce`, 16 fragments
//!   `jobs/<n>` (au lieu d'un topic global unique portant tout) ;
//! - chaque message gossip est **validé avant relais** (`MessageValidator`) : un message
//!   invalide pénalise son émetteur (scoring gossipsub), il n'est pas relayé ;
//! - **limites par pair** (seaux à jetons à mémoire bornée) et limites de connexions ;
//! - rattrapage du journal en **requête-réponse point à point**, bornée (plus de réponse
//!   diffusée à tout le réseau) ;
//! - Kademlia ne sert que de **registre de fournisseurs** de contenu (aucun enregistrement).
//!
//! Ce crate ne connaît ni le journal ni le stockage : il reçoit des traits ([`Providers`]).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod behaviour;
mod clock;
mod command;
mod driver;
mod error;
mod event;
mod gossip_in;
mod handle;
mod identity;
mod limits;
mod messages;
mod presence;
mod providers;
mod ratelimit;
mod requests_in;
mod scoring;
mod start;
mod swarm_events;
mod topics;
mod wire;

pub use error::NetError;
pub use event::NetEvent;
pub use handle::NetworkHandle;
pub use identity::keypair_from_secret;
pub use limits::NetLimits;
pub use messages::{ContentRequest, ContentResponse, LedgerRequest, LedgerResponse};
pub use presence::{
    Presence, PresenceGate, ROLE_CLAIM, ROLE_COMPUTE, ROLE_STORAGE, ROLE_VERIFY,
};
pub use providers::{
    AcceptAll, ContentProvider, LedgerProvider, MessageValidator, NoContent, NoLedger, Providers,
    Verdict,
};
pub use start::{NetConfig, Network, start};
pub use topics::{SHARDS, Topic};
pub use wire::Bytes;

pub use libp2p::{Multiaddr, PeerId, multiaddr::Protocol};

#[cfg(test)]
mod tests;
