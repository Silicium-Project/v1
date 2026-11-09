use crate::wallet_text::{cancel_text, job_published_text};
use crate::{EventBody, LedgerError};
use serde::{Deserialize, Serialize};
use sil_codec::{decode, encode};
use sil_crypto::{Domain, SigningIdentity, sha256_domain, verify, verify_raw};
use sil_types::{Hash32, JobId, PublicKey, Signature};

pub const EVENT_VERSION: u8 = 1;

/// `previous` du premier événement d'une chaîne.
pub const NO_PREVIOUS: Hash32 = Hash32::from_bytes([0; 32]);

/// Position d'un événement dans la chaîne (auteur, job).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub job: JobId,
    pub sequence: u64,
    pub previous: Hash32,
    /// Horloge de l'auteur, informative (les baux sont jugés sur l'heure locale du lecteur).
    pub issued_at_ms: u64,
}

/// Événement signé : maillon de la chaîne `(author, job)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub version: u8,
    pub author: PublicKey,
    pub job: JobId,
    pub sequence: u64,
    pub previous: Hash32,
    pub issued_at_ms: u64,
    pub body: EventBody,
    pub signature: Signature,
}

#[derive(Serialize)]
struct Core<'a> {
    version: u8,
    author: &'a PublicKey,
    job: &'a JobId,
    sequence: u64,
    previous: &'a Hash32,
    issued_at_ms: u64,
    body: &'a EventBody,
}

impl Event {
    /// Octets signés de l'événement (sans la signature), domaine `Event`.
    pub fn core_bytes(&self) -> Result<Vec<u8>, LedgerError> {
        Ok(encode(&Core {
            version: self.version,
            author: &self.author,
            job: &self.job,
            sequence: self.sequence,
            previous: &self.previous,
            issued_at_ms: self.issued_at_ms,
            body: &self.body,
        })?)
    }

    /// Construit et signe un événement.
    pub fn sign(
        identity: &SigningIdentity,
        header: Header,
        body: EventBody,
    ) -> Result<Self, LedgerError> {
        let mut event = Self {
            version: EVENT_VERSION,
            author: identity.public(),
            job: header.job,
            sequence: header.sequence,
            previous: header.previous,
            issued_at_ms: header.issued_at_ms,
            body,
            signature: Signature::from_bytes([0; 64]),
        };
        event.signature = match event.signing_text() {
            Some(text) => identity.sign_raw(text.as_bytes()),
            None => identity.sign(Domain::Event, &event.core_bytes()?),
        };
        Ok(event)
    }

    /// Version et signature.
    pub fn verify_signature(&self) -> Result<(), LedgerError> {
        if self.version != EVENT_VERSION {
            return Err(LedgerError::BadVersion);
        }
        match self.signing_text() {
            Some(text) => verify_raw(&self.author, text.as_bytes(), &self.signature),
            None => verify(
                &self.author,
                Domain::Event,
                &self.core_bytes()?,
                &self.signature,
            ),
        }
        .map_err(|_| LedgerError::BadSignature)
    }

    /// The text a wallet signs for this event, for a `JobPublished` (requester event). Other
    /// events are signed by node keys over the binary frame, so they have no text.
    pub fn signing_text(&self) -> Option<String> {
        match &self.body {
            EventBody::JobPublished { spec } => Some(job_published_text(
                &self.author,
                &self.job,
                self.sequence,
                &self.previous,
                self.issued_at_ms,
                &spec.signature,
            )),
            EventBody::Cancel => Some(cancel_text(
                &self.author,
                &self.job,
                self.sequence,
                &self.previous,
                self.issued_at_ms,
            )),
            _ => None,
        }
    }

    /// Octets canoniques de l'événement complet (réseau, disque).
    pub fn to_bytes(&self) -> Result<Vec<u8>, LedgerError> {
        Ok(encode(self)?)
    }

    /// Décode des octets reçus (taille et canonicité vérifiées).
    pub fn from_bytes(bytes: &[u8], max_len: usize) -> Result<Self, LedgerError> {
        Ok(decode(bytes, max_len)?)
    }

    /// Identifiant (hash) et taille encodée.
    pub fn id_and_len(&self) -> Result<(Hash32, usize), LedgerError> {
        let bytes = self.to_bytes()?;
        Ok((sha256_domain(Domain::EventId, &bytes), bytes.len()))
    }

    /// Identifiant (hash) de l'événement.
    pub fn id(&self) -> Result<Hash32, LedgerError> {
        Ok(self.id_and_len()?.0)
    }
}
