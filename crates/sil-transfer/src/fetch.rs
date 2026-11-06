use crate::TransferError;
use futures::future::join_all;
use sil_content::{ContentError, ContentStore, Download, Manifest};
use sil_net::{Bytes, ContentRequest, ContentResponse, NetworkHandle, PeerId};
use sil_types::ContentId;

#[derive(Debug, Clone)]
pub struct TransferConfig {
    /// Morceaux demandés en parallèle à un même pair.
    pub window: usize,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self { window: 4 }
    }
}

/// Pourquoi un pair est abandonné (texte pour les journaux et l'erreur finale).
type PeerFailure = String;

/// Télécharge `id` depuis `peers`, dans l'ordre. Sans effet si le contenu est déjà là.
///
/// Un pair qui répond mal (manifeste faux, morceau faux, absent, saturé) est abandonné et
/// le suivant continue **là où le précédent s'est arrêté**.
pub async fn fetch(
    net: &NetworkHandle,
    store: &ContentStore,
    id: &ContentId,
    peers: &[PeerId],
    config: &TransferConfig,
) -> Result<(), TransferError> {
    if store.has(id) {
        return Ok(());
    }
    if peers.is_empty() {
        return Err(TransferError::NoPeers);
    }
    let mut download: Option<Download> = None;
    let mut last = String::new();
    for peer in peers {
        match from_peer(net, store, id, *peer, &mut download, config).await {
            Ok(()) => {
                if let Some(done) = download.take() {
                    done.finish()?;
                }
                // Les autres pairs peuvent maintenant nous trouver.
                let _ = net.provide(*id).await;
                return Ok(());
            }
            Err(failure) => {
                tracing::warn!(%peer, %failure, "pair abandonné pour ce contenu");
                last = failure;
            }
        }
    }
    Err(TransferError::Exhausted { last })
}

/// Complète le téléchargement auprès d'un pair. `Ok` = tous les morceaux sont écrits
/// (ou le contenu est déjà complet : `download` reste alors `None`).
async fn from_peer(
    net: &NetworkHandle,
    store: &ContentStore,
    id: &ContentId,
    peer: PeerId,
    download: &mut Option<Download>,
    config: &TransferConfig,
) -> Result<(), PeerFailure> {
    if download.is_none() {
        let manifest = ask_manifest(net, peer, id).await?;
        match store.begin_download(id, manifest) {
            Ok(Some(started)) => *download = Some(started),
            Ok(None) => return Ok(()),
            Err(error @ ContentError::WrongId) => return Err(format!("manifeste faux : {error}")),
            Err(error) => return Err(error.to_string()),
        }
    }
    let Some(active) = download.as_mut() else {
        return Ok(());
    };
    loop {
        let wanted: Vec<u32> = active
            .missing()
            .into_iter()
            .take(config.window.max(1))
            .collect();
        if wanted.is_empty() {
            return Ok(());
        }
        let answers = join_all(wanted.iter().map(|index| ask_chunk(net, peer, id, *index))).await;
        for (index, answer) in wanted.into_iter().zip(answers) {
            let data = answer?;
            active
                .write_chunk(index, &data)
                .map_err(|e| format!("morceau {index} : {e}"))?;
        }
    }
}

async fn ask_manifest(
    net: &NetworkHandle,
    peer: PeerId,
    id: &ContentId,
) -> Result<Manifest, PeerFailure> {
    let request = ContentRequest::Manifest { id: *id };
    match net.request_content(peer, request).await {
        Ok(ContentResponse::Manifest(Bytes(bytes))) => {
            Manifest::from_bytes(&bytes).map_err(|e| e.to_string())
        }
        Ok(ContentResponse::NotFound) => Err("contenu absent".into()),
        Ok(ContentResponse::Busy) => Err("pair saturé".into()),
        Ok(ContentResponse::Chunk(_)) => Err("réponse inattendue".into()),
        Err(error) => Err(error.to_string()),
    }
}

async fn ask_chunk(
    net: &NetworkHandle,
    peer: PeerId,
    id: &ContentId,
    index: u32,
) -> Result<Vec<u8>, PeerFailure> {
    let request = ContentRequest::Chunk { id: *id, index };
    match net.request_content(peer, request).await {
        Ok(ContentResponse::Chunk(Bytes(bytes))) => Ok(bytes),
        Ok(ContentResponse::NotFound) => Err(format!("morceau {index} absent")),
        Ok(ContentResponse::Busy) => Err("pair saturé".into()),
        Ok(ContentResponse::Manifest(_)) => Err("réponse inattendue".into()),
        Err(error) => Err(error.to_string()),
    }
}
