//! Aiguillage des événements des sous-comportements libp2p.

use crate::{behaviour::BehaviourEvent, driver::Driver};
use libp2p::{gossipsub, identify, kad};

impl Driver {
    pub(crate) fn on_behaviour(&mut self, event: BehaviourEvent) {
        match event {
            BehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source,
                message_id,
                message,
            }) => self.on_gossip(propagation_source, &message_id, message),
            BehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. }) => {
                for address in info.listen_addrs {
                    self.swarm
                        .behaviour_mut()
                        .kad
                        .add_address(&peer_id, address);
                }
            }
            BehaviourEvent::Kad(kad::Event::OutboundQueryProgressed {
                id, result, step, ..
            }) => self.on_kad_query(id, result, step.last),
            BehaviourEvent::Ledger(event) => self.on_ledger_event(event),
            BehaviourEvent::Content(event) => self.on_content_event(event),
            _ => {}
        }
    }

    /// Cumule les fournisseurs trouvés ; répond quand la recherche est terminée.
    fn on_kad_query(&mut self, id: kad::QueryId, result: kad::QueryResult, last: bool) {
        if let kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
            providers,
            ..
        })) = result
            && let Some((found, _)) = self.pending_providers.get_mut(&id)
        {
            found.extend(providers);
        }
        if last && let Some((found, reply)) = self.pending_providers.remove(&id) {
            let _ = reply.send(Ok(found.into_iter().collect()));
        }
    }
}
