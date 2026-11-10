//! Options de traitement d'un lot (zone de 16 octets de l'en-tête version 2).
//!
//! Elles vivent dans le lot, donc dans l'entrée du job : leur hash est engagé avec les octets. Une
//! option absente vaut 0, et chaque option n'a qu'une forme valide : deux lots qui font la même
//! chose ont les mêmes octets.

use crate::{BatchError, Kind};

/// Taille de la zone d'options : quatre octets d'image, deux u32 et un octet d'audio, trois octets
/// réservés (à zéro).
pub const OPTIONS_LEN: usize = 16;

/// Redimensionnement : étirer à la taille exacte (défaut, comportement de la version 1).
pub const FIT_STRETCH: u8 = 0;
/// Redimensionnement : tenir dans la boîte en gardant les proportions.
pub const FIT_CONTAIN: u8 = 1;
/// Redimensionnement : remplir la boîte en gardant les proportions, puis recadrer au centre.
pub const FIT_COVER: u8 = 2;

pub const FLAG_GRAYSCALE: u8 = 1;
pub const FLAG_FLIP_HORIZONTAL: u8 = 2;
pub const FLAG_FLIP_VERTICAL: u8 = 4;

/// Débits audio acceptés, en kbps (0 = défaut du moteur).
pub const MIN_BITRATE_KBPS: u32 = 32;
pub const MAX_BITRATE_KBPS: u32 = 320;
/// Fréquences d'échantillonnage acceptées, en Hz (0 = garder celle de la source).
pub const SAMPLE_RATES_HZ: [u32; 8] = [8000, 11025, 16000, 22050, 24000, 32000, 44100, 48000];

/// Options d'un lot. Les champs image ne valent 0 que pour un lot audio, et inversement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Options {
    /// Qualité JPEG de 1 à 100 ; 0 = défaut de l'encodeur. Image seulement.
    pub quality: u8,
    /// Mode de redimensionnement (`FIT_*`). Image seulement.
    pub fit: u8,
    /// Rotation en quarts de tour : 0, 1 (90°), 2 (180°), 3 (270°). Image seulement.
    pub rotate: u8,
    /// Indicateurs (`FLAG_*`). Image seulement.
    pub flags: u8,
    /// Débit audio en kbps ; 0 = défaut du moteur. Audio seulement.
    pub bitrate_kbps: u32,
    /// Fréquence d'échantillonnage audio en Hz ; 0 = garder. Audio seulement.
    pub sample_rate_hz: u32,
    /// Canaux audio : 0 = garder, 1 = mono, 2 = stéréo. Audio seulement.
    pub channels: u8,
}

impl Options {
    /// Écrit la zone d'options (16 octets) à la suite de `out`.
    pub(crate) fn write(&self, out: &mut Vec<u8>) {
        out.push(self.quality);
        out.push(self.fit);
        out.push(self.rotate);
        out.push(self.flags);
        out.extend_from_slice(&self.bitrate_kbps.to_le_bytes());
        out.extend_from_slice(&self.sample_rate_hz.to_le_bytes());
        out.push(self.channels);
        out.extend_from_slice(&[0; 3]);
    }

    /// Lit une zone de 16 octets. Les contrôles de forme sont dans [`Options::validate`].
    pub(crate) fn read(bytes: &[u8]) -> Self {
        let word = |at: usize| {
            let mut value = [0u8; 4];
            value.copy_from_slice(&bytes[at..at + 4]);
            u32::from_le_bytes(value)
        };
        Self {
            quality: bytes[0],
            fit: bytes[1],
            rotate: bytes[2],
            flags: bytes[3],
            bitrate_kbps: word(4),
            sample_rate_hz: word(8),
            channels: bytes[12],
        }
    }

    /// Refuse une option hors de sa plage, ou une option d'un autre type de média.
    pub fn validate(&self, kind: Kind) -> Result<(), BatchError> {
        match kind {
            Kind::Image => {
                if self.bitrate_kbps != 0 || self.sample_rate_hz != 0 || self.channels != 0 {
                    return Err(BatchError::BadOption("option audio sur un lot d'images"));
                }
                if self.quality > 100 {
                    return Err(BatchError::BadOption("qualité JPEG hors de 0 à 100"));
                }
                if self.fit > FIT_COVER {
                    return Err(BatchError::BadOption("mode de redimensionnement inconnu"));
                }
                if self.rotate > 3 {
                    return Err(BatchError::BadOption("rotation hors de 0 à 3"));
                }
                if self.flags & !(FLAG_GRAYSCALE | FLAG_FLIP_HORIZONTAL | FLAG_FLIP_VERTICAL) != 0 {
                    return Err(BatchError::BadOption("indicateur inconnu"));
                }
            }
            Kind::Audio => {
                if self.quality != 0 || self.fit != 0 || self.rotate != 0 || self.flags != 0 {
                    return Err(BatchError::BadOption("option image sur un lot audio"));
                }
                // Transcodage sans perte seulement : débit, fréquence et canaux changent les octets
                // d'un moteur à l'autre, donc ne sont pas vérifiables par recalcul.
                if self.bitrate_kbps != 0 || self.sample_rate_hz != 0 || self.channels != 0 {
                    return Err(BatchError::BadOption(
                        "débit, fréquence et canaux non pris en charge : transcodage sans perte",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_zone_round_trips() {
        let options = Options {
            quality: 80,
            fit: FIT_COVER,
            rotate: 3,
            flags: FLAG_GRAYSCALE | FLAG_FLIP_VERTICAL,
            bitrate_kbps: 0,
            sample_rate_hz: 0,
            channels: 0,
        };
        let mut bytes = Vec::new();
        options.write(&mut bytes);
        assert_eq!(bytes.len(), OPTIONS_LEN);
        assert_eq!(Options::read(&bytes), options);
    }

    #[test]
    fn an_audio_option_on_an_image_batch_is_refused() {
        let options = Options {
            channels: 2,
            ..Options::default()
        };
        assert!(options.validate(Kind::Image).is_err());
        assert!(options.validate(Kind::Audio).is_err());
    }

    #[test]
    fn out_of_range_values_are_refused() {
        let quality = Options {
            quality: 101,
            ..Options::default()
        };
        assert!(quality.validate(Kind::Image).is_err());
        let rate = Options {
            sample_rate_hz: 12345,
            ..Options::default()
        };
        assert!(rate.validate(Kind::Audio).is_err());
        let bitrate = Options {
            bitrate_kbps: 400,
            ..Options::default()
        };
        assert!(bitrate.validate(Kind::Audio).is_err());
    }
}
