use crate::shared::{Shared, lock};
use sil_transfer::{TransferConfig, fetch};
use sil_types::ContentId;
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

/// S'assure qu'un contenu est dans le magasin local, en le téléchargeant des pairs si besoin
/// (fournisseurs annoncés d'abord, puis tous les pairs connectés).
pub(crate) async fn ensure_content(shared: &Arc<Shared>, id: &ContentId) -> bool {
    if shared.store.has(id) {
        return true;
    }
    // Un seul téléchargement à la fois par contenu : deux fragments qui partagent la même
    // entrée ne doivent pas écrire le même fichier partiel en parallèle.
    let gate = lock(&shared.work).fetching.entry(*id).or_default().clone();
    let _turn = gate.lock().await;
    if shared.store.has(id) {
        lock(&shared.work).fetching.remove(id);
        return true;
    }
    let fetched = download(shared, id).await;
    if fetched {
        lock(&shared.work).fetching.remove(id);
    }
    fetched
}

async fn download(shared: &Arc<Shared>, id: &ContentId) -> bool {
    let Some(net) = shared.net.get() else {
        return false;
    };
    let mut peers = timeout(Duration::from_secs(3), net.find_providers(*id))
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();
    for peer in net.peers().await.unwrap_or_default() {
        if !peers.contains(&peer) {
            peers.push(peer);
        }
    }
    fetch(net, &shared.store, id, &peers, &TransferConfig::default())
        .await
        .is_ok()
}
