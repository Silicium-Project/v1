use crate::Topic;
use libp2p::PeerId;

/// Ce que le réseau remonte à l'application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetEvent {
    /// Message gossip **déjà validé** par le `MessageValidator`.
    Gossip {
        from: PeerId,
        topic: Topic,
        data: Vec<u8>,
    },
    Connected(PeerId),
    Disconnected(PeerId),
}
