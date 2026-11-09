use crate::{
    ContentRequest, ContentResponse, LedgerRequest, LedgerResponse, NetError, NetLimits, Topic,
    limits::{CHUNK_RESPONSE_MAX, MANIFEST_RESPONSE_MAX, REQUEST_MAX},
    scoring,
};
use libp2p::{
    PeerId, StreamProtocol, Swarm, SwarmBuilder, connection_limits, dcutr,
    gossipsub::{self, MessageId, ValidationMode},
    identify, kad, noise, relay,
    request_response::{self, ProtocolSupport, cbor},
    swarm::{NetworkBehaviour, behaviour::toggle::Toggle},
    tcp, yamux,
};
use sil_crypto::sha256;
use std::time::Duration;

pub(crate) const LEDGER_PROTOCOL: &str = "/sil/ledger/1";
pub(crate) const CONTENT_PROTOCOL: &str = "/sil/content/1";

#[derive(NetworkBehaviour)]
pub(crate) struct Behaviour {
    pub(crate) limits: connection_limits::Behaviour,
    pub(crate) gossipsub: gossipsub::Behaviour,
    pub(crate) kad: kad::Behaviour<kad::store::MemoryStore>,
    pub(crate) identify: identify::Behaviour,
    pub(crate) ledger: cbor::Behaviour<LedgerRequest, LedgerResponse>,
    pub(crate) content: cbor::Behaviour<ContentRequest, ContentResponse>,
    /// Réservation auprès d'un relais : le nœud devient joignable via `relais/p2p-circuit`.
    pub(crate) relay_client: relay::client::Behaviour,
    /// Relais public (nœud joignable directement) : seulement si `NetConfig::relay_server`.
    pub(crate) relay_server: Toggle<relay::Behaviour>,
    /// Passage de la connexion relayée à une connexion directe (perforation de NAT).
    pub(crate) dcutr: dcutr::Behaviour,
}

/// Bornes d'un relais : assez de circuits pour un petit réseau, circuits longs (le contenu
/// transite par le relais). Les bornes par défaut (128 Kio, 2 min) couperaient les transferts.
pub(crate) fn relay_server_config() -> relay::Config {
    relay::Config {
        max_reservations: 512,
        max_reservations_per_peer: 2,
        max_circuits: 256,
        max_circuits_per_peer: 16,
        max_circuit_duration: Duration::from_secs(60 * 60),
        max_circuit_bytes: 1 << 30,
        ..relay::Config::default()
    }
}

fn config_error(error: impl ToString) -> NetError {
    NetError::Config(error.to_string())
}

/// Identifiant de message = SHA-256 de (émetteur, séquence, données) : pas de collision
/// fabricable pour supprimer le message d'un autre (V0 : `DefaultHasher`, non cryptographique).
fn message_id(message: &gossipsub::Message) -> MessageId {
    let mut data = Vec::with_capacity(message.data.len() + 48);
    if let Some(source) = &message.source {
        data.extend_from_slice(&source.to_bytes());
    }
    data.extend_from_slice(&message.sequence_number.unwrap_or(0).to_le_bytes());
    data.extend_from_slice(&message.data);
    MessageId::from(sha256(&data).as_bytes().to_vec())
}

fn gossip(
    keypair: &libp2p::identity::Keypair,
    limits: &NetLimits,
) -> Result<gossipsub::Behaviour, NetError> {
    let mut builder = gossipsub::ConfigBuilder::default();
    builder
        .heartbeat_interval(Duration::from_secs(1))
        .validation_mode(ValidationMode::Strict)
        .validate_messages()
        .max_transmit_size(limits.max_event_message_bytes)
        .max_transmit_size_for_topic(limits.max_presence_bytes, Topic::Presence.ident().hash())
        .message_id_fn(message_id);
    let config = builder.build().map_err(config_error)?;
    let mut behaviour = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(keypair.clone()),
        config,
    )
    .map_err(config_error)?;
    let (params, thresholds) = scoring::params();
    behaviour
        .with_peer_score(params, thresholds)
        .map_err(config_error)?;
    Ok(behaviour)
}

fn kademlia(local: PeerId) -> kad::Behaviour<kad::store::MemoryStore> {
    // `max_records: 0` : aucun enregistrement n'est accepté (V0 stockait des enregistrements
    // de tâches jamais lus). Seuls les fournisseurs de contenu sont retenus, en nombre borné.
    let store = kad::store::MemoryStore::with_config(
        local,
        kad::store::MemoryStoreConfig {
            max_records: 0,
            max_provided_keys: 4096,
            ..Default::default()
        },
    );
    let mut behaviour = kad::Behaviour::new(local, store);
    behaviour.set_mode(Some(kad::Mode::Server));
    behaviour
}

fn ledger(limits: &NetLimits) -> cbor::Behaviour<LedgerRequest, LedgerResponse> {
    let codec = cbor::codec::Codec::default()
        .set_request_size_maximum(REQUEST_MAX)
        .set_response_size_maximum(
            (limits.max_ledger_events * limits.max_event_message_bytes) as u64,
        );
    request_response::Behaviour::with_codec(
        codec,
        [(StreamProtocol::new(LEDGER_PROTOCOL), ProtocolSupport::Full)],
        request_response::Config::default().with_request_timeout(limits.request_timeout),
    )
}

fn content(limits: &NetLimits) -> cbor::Behaviour<ContentRequest, ContentResponse> {
    let codec = cbor::codec::Codec::default()
        .set_request_size_maximum(REQUEST_MAX)
        .set_response_size_maximum(MANIFEST_RESPONSE_MAX.max(CHUNK_RESPONSE_MAX));
    request_response::Behaviour::with_codec(
        codec,
        [(StreamProtocol::new(CONTENT_PROTOCOL), ProtocolSupport::Full)],
        request_response::Config::default().with_request_timeout(limits.request_timeout),
    )
}

pub(crate) fn build(
    keypair: libp2p::identity::Keypair,
    limits: &NetLimits,
    relay_server: bool,
) -> Result<Swarm<Behaviour>, NetError> {
    let connection_limits = connection_limits::ConnectionLimits::default()
        .with_max_established(Some(limits.max_established))
        .with_max_established_per_peer(Some(limits.max_per_peer))
        .with_max_pending_incoming(Some(limits.max_pending_incoming));
    let swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(config_error)?
        .with_quic()
        .with_relay_client(noise::Config::new, yamux::Config::default)
        .map_err(config_error)?
        .with_behaviour(|key, relay_client| {
            let local = key.public().to_peer_id();
            Ok(Behaviour {
                limits: connection_limits::Behaviour::new(connection_limits),
                gossipsub: gossip(key, limits)?,
                kad: kademlia(local),
                identify: identify::Behaviour::new(identify::Config::new(
                    "/sil/1.0.0".into(),
                    key.public(),
                )),
                ledger: ledger(limits),
                content: content(limits),
                relay_client,
                relay_server: Toggle::from(
                    relay_server.then(|| relay::Behaviour::new(local, relay_server_config())),
                ),
                dcutr: dcutr::Behaviour::new(local),
            })
        })
        .map_err(config_error)?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();
    Ok(swarm)
}
