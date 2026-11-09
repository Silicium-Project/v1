//! Client de l'API publique : connexion par wallet (signature Ed25519), comité, contenu,
//! soumission de l'événement signé, état du job. Même chemin que le site.

use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sil_crypto::SigningIdentity;
use sil_types::{JobId, PublicKey};
use solana_sdk::pubkey::Pubkey;

pub struct Api {
    base: String,
    token: String,
}

impl Api {
    /// Connexion comme un wallet : l'API donne un message, l'identité le signe.
    pub fn login(base: &str, identity: &SigningIdentity) -> Result<Self, String> {
        let address = address_of(&identity.public());
        let base = base.trim_end_matches('/').to_string();
        let challenge = post(
            &base,
            "/v1/auth/wallet/challenge",
            None,
            &json!({ "address": address }),
        )?;
        let message = text(&challenge, "message")?;
        let signature = identity.sign_raw(message.as_bytes());
        let body = json!({
            "address": address,
            "nonce": text(&challenge, "nonce")?,
            "signature": STANDARD.encode(signature.as_bytes()),
        });
        let login = post(&base, "/v1/auth/wallet/login", None, &body)?;
        Ok(Self {
            token: text(&login, "token")?,
            base,
        })
    }

    /// Comité configuré sur l'API et seuil à la majorité stricte.
    /// Envoie l'entrée au réseau (le nœud la stocke par hash).
    pub fn upload(&self, data: &[u8]) -> Result<(), String> {
        let url = format!("{}/v1/content", self.base);
        ureq::put(&url)
            .set("Authorization", &format!("Bearer {}", self.token))
            .send_bytes(data)
            .map(|_| ())
            .map_err(describe)
    }

    /// Soumet l'événement `JobPublished` signé par le requester.
    pub fn publish(&self, job: &JobId, event: &[u8]) -> Result<(), String> {
        let body = json!({
            "job_id": job.to_hex(),
            "event": STANDARD.encode(event),
            "label": "test devnet",
        });
        post(&self.base, "/v1/jobs", Some(&self.token), &body).map(|_| ())
    }

    /// État du job tel que le réseau le rapporte (`running`, `finalized`, ...).
    pub fn state(&self, job: &JobId) -> Result<String, String> {
        let path = format!("/v1/jobs/{}", job.to_hex());
        let reply = get(&self.base, &path, &self.token)?;
        text(&reply["status"], "state")
    }
}

/// Adresse Solana d'une clé du ledger : même clé ed25519, donc même adresse.
pub fn address_of(key: &PublicKey) -> String {
    Pubkey::new_from_array(*key.as_bytes()).to_string()
}

fn get(base: &str, path: &str, token: &str) -> Result<Value, String> {
    ureq::get(&format!("{base}{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(describe)?
        .into_json()
        .map_err(|e| e.to_string())
}

fn post(base: &str, path: &str, token: Option<&str>, body: &Value) -> Result<Value, String> {
    let mut request = ureq::post(&format!("{base}{path}"));
    if let Some(token) = token {
        request = request.set("Authorization", &format!("Bearer {token}"));
    }
    request
        .send_json(body.clone())
        .map_err(describe)?
        .into_json()
        .map_err(|e| e.to_string())
}

fn text(value: &Value, field: &str) -> Result<String, String> {
    value[field]
        .as_str()
        .map(str::to_string)
        .ok_or(format!("champ « {field} » attendu"))
}

/// Erreur HTTP lisible : code et message de l'API, jamais de jeton.
fn describe(error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, response) => {
            format!(
                "HTTP {code} : {}",
                response.into_string().unwrap_or_default()
            )
        }
        ureq::Error::Transport(transport) => transport.to_string(),
    }
}
