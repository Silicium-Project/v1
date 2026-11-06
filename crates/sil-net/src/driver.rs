//! Boucle réseau : un seul propriétaire du `Swarm`, piloté par des commandes typées.
//! Les gestionnaires d'événements sont dans `swarm_events.rs`, `gossip_in.rs`,
//! `requests_in.rs` (autres blocs `impl Driver`).

use crate::{
    NetError, NetEvent, NetLimits, Providers,
    behaviour::{Behaviour, BehaviourEvent},
    clock::Clock,
    command::{Command, Reply},
    messages::{ContentResponse, LedgerResponse},
    ratelimit::PeerLimiters,
};
use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, Swarm,
    kad::{self, RecordKey},
    multiaddr::Protocol,
    request_response::OutboundRequestId,
    swarm::SwarmEvent,
};
use std::{
    collections::{BTreeSet, HashMap},
    time::Duration,
};
use tokio::sync::mpsc;

/// Délai entre deux tentatives de reconnexion aux pairs d'amorçage.
const REDIAL_EVERY: Duration = Duration::from_secs(10);

pub(crate) struct Driver {
    pub(crate) swarm: Swarm<Behaviour>,
    pub(crate) commands: mpsc::Receiver<Command>,
    pub(crate) events: mpsc::Sender<NetEvent>,
    pub(crate) providers: Providers,
    pub(crate) limits: NetLimits,
    pub(crate) limiters: PeerLimiters,
    pub(crate) clock: Clock,
    pub(crate) listen_addrs: Vec<Multiaddr>,
    pub(crate) bootstrap: Vec<Multiaddr>,
    pub(crate) pending_ledger: HashMap<OutboundRequestId, Reply<LedgerResponse>>,
    pub(crate) pending_content: HashMap<OutboundRequestId, Reply<ContentResponse>>,
    pub(crate) pending_providers: HashMap<kad::QueryId, (BTreeSet<PeerId>, Reply<Vec<PeerId>>)>,
}

/// Termine une requête sortante en prévenant celui qui attend.
pub(crate) fn complete<T>(
    pending: &mut HashMap<OutboundRequestId, Reply<T>>,
    id: OutboundRequestId,
    result: Result<T, NetError>,
) {
    if let Some(reply) = pending.remove(&id) {
        let _ = reply.send(result);
    }
}

fn peer_of(address: &Multiaddr) -> Option<PeerId> {
    address.iter().find_map(|part| match part {
        Protocol::P2p(peer) => Some(peer),
        _ => None,
    })
}

impl Driver {
    pub(crate) async fn run(mut self) {
        let mut redial = tokio::time::interval(REDIAL_EVERY);
        redial.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                command = self.commands.recv() => match command {
                    Some(Command::Shutdown) | None => break,
                    Some(command) => self.on_command(command),
                },
                event = self.swarm.select_next_some() => self.on_swarm_event(event),
                _ = redial.tick() => self.redial_bootstrap(),
            }
        }
    }

    /// Remonte un événement à l'application ; si elle est à la traîne, l'événement est
    /// abandonné (la boucle réseau ne doit jamais attendre l'application).
    pub(crate) fn emit(&self, event: NetEvent) {
        if self.events.try_send(event).is_err() {
            tracing::warn!("file d'événements réseau pleine ou fermée : événement abandonné");
        }
    }

    fn redial_bootstrap(&mut self) {
        for address in self.bootstrap.clone() {
            let connected = peer_of(&address).is_some_and(|peer| self.swarm.is_connected(&peer));
            if !connected {
                let _ = self.swarm.dial(address);
            }
        }
    }

    fn on_command(&mut self, command: Command) {
        match command {
            Command::Publish { topic, data, reply } => {
                let result = self
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(topic.ident(), data)
                    .map(|_| ())
                    .map_err(|e| NetError::Publish(e.to_string()));
                let _ = reply.send(result);
            }
            Command::Subscribe { topic, reply } => {
                let result = self
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .subscribe(&topic.ident())
                    .map(|_| ())
                    .map_err(|e| NetError::Publish(e.to_string()));
                let _ = reply.send(result);
            }
            Command::Unsubscribe { topic } => {
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .unsubscribe(&topic.ident());
            }
            Command::Ledger {
                peer,
                request,
                reply,
            } => {
                let id = self
                    .swarm
                    .behaviour_mut()
                    .ledger
                    .send_request(&peer, request);
                self.pending_ledger.insert(id, reply);
            }
            Command::Content {
                peer,
                request,
                reply,
            } => {
                let id = self
                    .swarm
                    .behaviour_mut()
                    .content
                    .send_request(&peer, request);
                self.pending_content.insert(id, reply);
            }
            Command::Provide { id } => {
                let key = RecordKey::new(id.as_bytes());
                if let Err(error) = self.swarm.behaviour_mut().kad.start_providing(key) {
                    tracing::warn!(%error, "annonce de contenu impossible");
                }
            }
            Command::FindProviders { id, reply } => {
                let key = RecordKey::new(id.as_bytes());
                let query = self.swarm.behaviour_mut().kad.get_providers(key);
                self.pending_providers
                    .insert(query, (BTreeSet::new(), reply));
            }
            Command::Dial { address, reply } => {
                let result = self
                    .swarm
                    .dial(address)
                    .map_err(|e| NetError::Dial(e.to_string()));
                let _ = reply.send(result);
            }
            Command::ListenAddrs { reply } => {
                let _ = reply.send(self.listen_addrs.clone());
            }
            Command::Peers { reply } => {
                let _ = reply.send(self.swarm.connected_peers().copied().collect());
            }
            Command::Shutdown => {}
        }
    }

    pub(crate) fn on_swarm_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => self.listen_addrs.push(address),
            SwarmEvent::ExpiredListenAddr { address, .. } => {
                self.listen_addrs.retain(|known| *known != address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                self.emit(NetEvent::Connected(peer_id));
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                num_established: 0,
                ..
            } => self.emit(NetEvent::Disconnected(peer_id)),
            SwarmEvent::Behaviour(event) => self.on_behaviour(event),
            _ => {}
        }
    }
}
