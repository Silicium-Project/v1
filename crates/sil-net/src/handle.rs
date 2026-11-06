use crate::{
    ContentRequest, ContentResponse, LedgerRequest, LedgerResponse, NetError, NetLimits, Topic,
    command::Command,
};
use libp2p::{Multiaddr, PeerId};
use sil_types::ContentId;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// Poignée de pilotage du réseau (clone bon marché, utilisable depuis n'importe quelle tâche).
#[derive(Clone)]
pub struct NetworkHandle {
    pub(crate) commands: mpsc::Sender<Command>,
    pub(crate) local: PeerId,
    pub(crate) limits: NetLimits,
}

/// Attente maximale d'une commande locale (la boucle répond vite ou est bloquée).
const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

impl NetworkHandle {
    pub fn local_peer_id(&self) -> PeerId {
        self.local
    }

    async fn send(&self, command: Command) -> Result<(), NetError> {
        self.commands
            .send(command)
            .await
            .map_err(|_| NetError::Closed)
    }

    async fn ask<T>(
        &self,
        build: impl FnOnce(oneshot::Sender<Result<T, NetError>>) -> Command,
    ) -> Result<T, NetError> {
        let (reply, answer) = oneshot::channel();
        self.send(build(reply)).await?;
        match tokio::time::timeout(COMMAND_TIMEOUT, answer).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(NetError::Closed),
            Err(_) => Err(NetError::Timeout),
        }
    }

    /// Publie un message sur un topic. Refusé localement s'il dépasse la taille du topic.
    pub async fn publish(&self, topic: Topic, data: Vec<u8>) -> Result<(), NetError> {
        let max = match topic {
            Topic::Presence => self.limits.max_presence_bytes,
            _ => self.limits.max_event_message_bytes,
        };
        if data.len() > max {
            return Err(NetError::TooLarge {
                len: data.len(),
                max,
            });
        }
        self.ask(|reply| Command::Publish { topic, data, reply })
            .await
    }

    pub async fn subscribe(&self, topic: Topic) -> Result<(), NetError> {
        self.ask(|reply| Command::Subscribe { topic, reply }).await
    }

    pub async fn unsubscribe(&self, topic: Topic) -> Result<(), NetError> {
        self.send(Command::Unsubscribe { topic }).await
    }

    /// Demande des événements à un pair (rattrapage).
    pub async fn request_ledger(
        &self,
        peer: PeerId,
        request: LedgerRequest,
    ) -> Result<LedgerResponse, NetError> {
        self.ask(|reply| Command::Ledger {
            peer,
            request,
            reply,
        })
        .await
    }

    /// Demande un manifeste ou un morceau de contenu à un pair.
    pub async fn request_content(
        &self,
        peer: PeerId,
        request: ContentRequest,
    ) -> Result<ContentResponse, NetError> {
        self.ask(|reply| Command::Content {
            peer,
            request,
            reply,
        })
        .await
    }

    /// Annonce que ce nœud détient `id`.
    pub async fn provide(&self, id: ContentId) -> Result<(), NetError> {
        self.send(Command::Provide { id }).await
    }

    /// Pairs qui annoncent détenir `id`.
    pub async fn find_providers(&self, id: ContentId) -> Result<Vec<PeerId>, NetError> {
        self.ask(|reply| Command::FindProviders { id, reply }).await
    }

    pub async fn dial(&self, address: Multiaddr) -> Result<(), NetError> {
        self.ask(|reply| Command::Dial { address, reply }).await
    }

    /// Adresses sur lesquelles le nœud écoute (vide tant que l'écoute n'a pas démarré).
    pub async fn listen_addrs(&self) -> Result<Vec<Multiaddr>, NetError> {
        let (reply, answer) = oneshot::channel();
        self.send(Command::ListenAddrs { reply }).await?;
        answer.await.map_err(|_| NetError::Closed)
    }

    /// Pairs connectés.
    pub async fn peers(&self) -> Result<Vec<PeerId>, NetError> {
        let (reply, answer) = oneshot::channel();
        self.send(Command::Peers { reply }).await?;
        answer.await.map_err(|_| NetError::Closed)
    }

    pub async fn shutdown(&self) {
        let _ = self.send(Command::Shutdown).await;
    }
}
