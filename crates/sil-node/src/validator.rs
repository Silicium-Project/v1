//! Validation des messages gossip **avant relais** (sans état).
//!
//! Rejeter = objectivement invalide (mal formé, signature fausse) : l'émetteur est pénalisé.
//! Un événement valide dont le job n'est pas encore connu est accepté ici (on ne le sait pas) ;
//! c'est le journal qui l'écartera sans pénalité.

use crate::wire::Envelope;
use sil_crypto::{Domain, verify};
use sil_net::{MessageValidator, Presence, Topic, Verdict};

pub struct NodeValidator;

impl MessageValidator for NodeValidator {
    fn validate(&self, topic: Topic, data: &[u8]) -> Verdict {
        match topic {
            Topic::Presence => {
                if Presence::from_bytes(data, 512).is_some() {
                    Verdict::Accept
                } else {
                    Verdict::Reject
                }
            }
            Topic::Announce | Topic::Jobs(_) => match Envelope::from_bytes(data) {
                Ok(Envelope::Event(event)) => {
                    if event.verify_signature().is_ok() {
                        Verdict::Accept
                    } else {
                        Verdict::Reject
                    }
                }
                Ok(Envelope::SettlementSig {
                    claim,
                    signer,
                    signature,
                }) => {
                    let valid =
                        verify(&signer, Domain::Settlement, &claim.bytes(), &signature).is_ok();
                    if valid {
                        Verdict::Accept
                    } else {
                        Verdict::Reject
                    }
                }
                Err(_) => Verdict::Reject,
            },
        }
    }
}
