//! Règlement par un relais : l'autorisation signée par les approbateurs part vers
//! `POST {relais}/v1/settlements` (CBOR), et le relais paie les frais. La lecture (escrow, statut,
//! réputation) reste sur le RPC. Le relais ne peut pas changer le règlement : le programme vérifie
//! les signatures des approbateurs. Il peut seulement refuser, et un autre relais reprend.

use sil_chain_core::Escrow;
use sil_settlement::{Authorization, Chain, ChainError, TxId, TxStatus};
use sil_settlement_solana::SolanaChain;
use sil_types::{JobId, PublicKey};
use serde_json::Value;
use std::time::Duration;

/// Délai d'une soumission au relais : au-delà, le règlement est « inconnu » et se réessaie.
const RELAY_TIMEOUT: Duration = Duration::from_secs(30);

pub struct RelayChain {
    reader: SolanaChain,
    relay: String,
    agent: ureq::Agent,
}

impl RelayChain {
    pub fn new(reader: SolanaChain, relay: &str) -> Self {
        Self {
            reader,
            relay: relay.trim_end_matches('/').to_string(),
            agent: ureq::AgentBuilder::new().timeout(RELAY_TIMEOUT).build(),
        }
    }
}

impl Chain for RelayChain {
    fn escrow(&self, job: &JobId) -> Result<Option<Escrow>, ChainError> {
        self.reader.escrow(job)
    }

    fn submit(&self, authorization: &Authorization) -> Result<TxId, ChainError> {
        let body = sil_codec::encode(authorization)
            .map_err(|e| ChainError::Rejected(format!("encodage du règlement : {e}")))?;
        let url = format!("{}/v1/settlements", self.relay);
        match self
            .agent
            .post(&url)
            .set("Content-Type", "application/cbor")
            .send_bytes(&body)
        {
            Ok(response) => {
                let answer: Value = response
                    .into_json()
                    .map_err(|e| ChainError::Transient(format!("réponse du relais : {e}")))?;
                answer["tx"]
                    .as_str()
                    .map(|tx| TxId(tx.to_string()))
                    .ok_or_else(|| ChainError::Transient("réponse du relais sans tx".into()))
            }
            Err(ureq::Error::Status(code, response)) => {
                let text = response
                    .into_json::<Value>()
                    .ok()
                    .and_then(|v| v["error"].as_str().map(str::to_string))
                    .unwrap_or_else(|| format!("statut {code}"));
                // 4xx : le relais a compris et refuse (règle, déjà réglé). Le reste : réessai.
                if (400..500).contains(&code) && code != 429 {
                    Err(ChainError::Rejected(text))
                } else {
                    Err(ChainError::Transient(text))
                }
            }
            Err(e) => Err(ChainError::Transient(e.to_string())),
        }
    }

    fn tx_status(&self, tx: &TxId) -> Result<TxStatus, ChainError> {
        self.reader.tx_status(tx)
    }

    fn verifier_standing(&self, verifier: &PublicKey) -> Result<Option<i64>, ChainError> {
        self.reader.verifier_standing(verifier)
    }
}
