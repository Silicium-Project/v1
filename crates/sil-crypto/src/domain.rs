/// Usage d'une signature. Le tag est ajouté devant le message signé : une signature
/// valide pour un domaine ne l'est jamais pour un autre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// Événement du journal.
    Event,
    /// Identifiant (hash) d'un événement.
    EventId,
    /// Identifiant (hash) d'un job.
    JobId,
    /// Spécification de job signée par le requester.
    JobSpec,
    /// Attestation d'un vérificateur.
    Attestation,
    /// Attestation d'un vérificateur en mode tolérance (embedding).
    AttestationTolerance,
    /// Certificat de finalisation.
    Certificate,
    /// Graine d'échantillonnage d'un vérificateur.
    Sampling,
    /// Autorisation de règlement d'un fragment (signée par chaque approbateur).
    Settlement,
    /// Présence d'un pair.
    Presence,
    /// Preuve de liaison compte ↔ machine.
    DeviceLink,
}

impl Domain {
    /// Tag unique, sans octet nul.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Event => "sil/v1/event",
            Self::EventId => "sil/v1/event-id",
            Self::JobId => "sil/v1/job-id",
            Self::JobSpec => "sil/v1/job-spec",
            Self::Attestation => "sil/v1/attestation",
            Self::AttestationTolerance => "sil/v1/attestation-tolerance",
            Self::Certificate => "sil/v1/certificate",
            Self::Sampling => "sil/v1/sampling",
            Self::Settlement => "sil/v1/settlement",
            Self::Presence => "sil/v1/presence",
            Self::DeviceLink => "sil/v1/device-link",
        }
    }

    /// Octets réellement signés : `tag || 0x00 || message`.
    pub(crate) fn frame(self, message: &[u8]) -> Vec<u8> {
        let tag = self.tag().as_bytes();
        let mut framed = Vec::with_capacity(tag.len() + 1 + message.len());
        framed.extend_from_slice(tag);
        framed.push(0);
        framed.extend_from_slice(message);
        framed
    }
}
