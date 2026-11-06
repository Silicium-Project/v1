use crate::NetLimits;
use libp2p::PeerId;
use sil_quota::KeyedLimiter;

/// Limiteurs par pair : gossip, rattrapage, contenu. Mémoire bornée (`max_tracked_peers`).
pub(crate) struct PeerLimiters {
    gossip: KeyedLimiter<PeerId>,
    ledger: KeyedLimiter<PeerId>,
    content: KeyedLimiter<PeerId>,
}

impl PeerLimiters {
    pub(crate) fn new(limits: &NetLimits) -> Self {
        let keys = limits.max_tracked_peers;
        Self {
            gossip: KeyedLimiter::new(keys, limits.gossip_burst, limits.gossip_per_sec),
            ledger: KeyedLimiter::new(keys, limits.ledger_burst, limits.ledger_per_sec),
            content: KeyedLimiter::new(keys, limits.content_burst, limits.content_per_sec),
        }
    }

    pub(crate) fn allow_gossip(&mut self, peer: &PeerId, now_ms: u64) -> bool {
        self.gossip.allow(peer, 1, now_ms)
    }

    pub(crate) fn allow_ledger(&mut self, peer: &PeerId, now_ms: u64) -> bool {
        self.ledger.allow(peer, 1, now_ms)
    }

    pub(crate) fn allow_content(&mut self, peer: &PeerId, now_ms: u64) -> bool {
        self.content.allow(peer, 1, now_ms)
    }
}
