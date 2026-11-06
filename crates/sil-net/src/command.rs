use crate::{ContentRequest, ContentResponse, LedgerRequest, LedgerResponse, NetError, Topic};
use libp2p::{Multiaddr, PeerId};
use sil_types::ContentId;
use tokio::sync::oneshot;

pub(crate) type Reply<T> = oneshot::Sender<Result<T, NetError>>;

/// Ordres envoyés à la boucle réseau.
pub(crate) enum Command {
    Publish {
        topic: Topic,
        data: Vec<u8>,
        reply: Reply<()>,
    },
    Subscribe {
        topic: Topic,
        reply: Reply<()>,
    },
    Unsubscribe {
        topic: Topic,
    },
    Ledger {
        peer: PeerId,
        request: LedgerRequest,
        reply: Reply<LedgerResponse>,
    },
    Content {
        peer: PeerId,
        request: ContentRequest,
        reply: Reply<ContentResponse>,
    },
    Provide {
        id: ContentId,
    },
    FindProviders {
        id: ContentId,
        reply: Reply<Vec<PeerId>>,
    },
    Dial {
        address: Multiaddr,
        reply: Reply<()>,
    },
    ListenAddrs {
        reply: oneshot::Sender<Vec<Multiaddr>>,
    },
    Peers {
        reply: oneshot::Sender<Vec<PeerId>>,
    },
    Shutdown,
}
