//! Conteneur de lot `SILBATCH` : une entrée de job qui porte plusieurs fichiers (images ou audio)
//! et l'opération à leur appliquer. Format v2, little-endian :
//!
//! ```text
//! magic "SILBATCH" (8) | version u8 = 2 | kind u8 | format u8 | reserved u8 = 0
//! width u32 | height u32 | count u32 | options (16 octets, voir `options`)
//! count x (len u32 | octets)
//! ```
//!
//! La version 1 (sans options) reste lisible : ses options valent toutes 0.
//!
//! Les paramètres vivent dans l'entrée : la spec du job ne porte rien d'autre. Le même lot donne
//! les mêmes octets sur toutes les machines, ce qui permet la vérification par recalcul.
//! Un fragment est une plage d'enregistrements `[start, end)`, codée par [`range_params`].

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod error;
mod frame;
mod options;

pub use error::BatchError;
pub use frame::{frame, unframe};
pub use options::{
    FIT_CONTAIN, FIT_COVER, FIT_STRETCH, FLAG_FLIP_HORIZONTAL, FLAG_FLIP_VERTICAL, FLAG_GRAYSCALE,
    MAX_BITRATE_KBPS, MIN_BITRATE_KBPS, OPTIONS_LEN, Options, SAMPLE_RATES_HZ,
};

const MAGIC: &[u8; 8] = b"SILBATCH";
const VERSION: u8 = 2;
const VERSION_1: u8 = 1;
/// Octets avant les options : magic, version, kind, format, réservé, largeur, hauteur, nombre.
const FIXED_LEN: usize = 8 + 4 + 12;
const HEADER_LEN: usize = FIXED_LEN + OPTIONS_LEN;
/// Plafonds : un lot ne tient pas plus d'enregistrements, ni d'enregistrement plus grand, que la
/// limite d'entrée d'un job.
pub const MAX_RECORDS: u32 = 65_536;
pub const MAX_RECORD_BYTES: u32 = 8 * 1024 * 1024;

/// Type de média du lot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Image,
    Audio,
}

impl Kind {
    fn code(self) -> u8 {
        match self {
            Kind::Image => 1,
            Kind::Audio => 2,
        }
    }

    fn from_code(code: u8) -> Result<Self, BatchError> {
        match code {
            1 => Ok(Kind::Image),
            2 => Ok(Kind::Audio),
            other => Err(BatchError::Kind(other)),
        }
    }
}

/// En-tête : ce qu'il faut appliquer, et combien d'enregistrements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub kind: Kind,
    /// Format de sortie (sens selon le type de média).
    pub format: u8,
    /// Largeur et hauteur cibles des images ; 0 = taille d'origine. Sans effet pour l'audio.
    pub width: u32,
    pub height: u32,
    pub count: u32,
    /// Options de traitement (voir [`Options`]).
    pub options: Options,
}

/// Un lot lu : son en-tête et les enregistrements, tranchés dans l'entrée (sans copie).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch<'a> {
    pub header: Header,
    pub records: Vec<&'a [u8]>,
}

/// Lit un lot. Refuse tout ce qui ne tient pas exactement dans `bytes`.
pub fn parse(bytes: &[u8]) -> Result<Batch<'_>, BatchError> {
    if bytes.len() < FIXED_LEN {
        return Err(BatchError::Truncated);
    }
    if &bytes[..8] != MAGIC {
        return Err(BatchError::Magic);
    }
    let (options, body) = match bytes[8] {
        VERSION_1 => (Options::default(), FIXED_LEN),
        VERSION => {
            if bytes.len() < HEADER_LEN {
                return Err(BatchError::Truncated);
            }
            if bytes[HEADER_LEN - 3..HEADER_LEN] != [0; 3] {
                return Err(BatchError::BadOption("zone réservée non nulle"));
            }
            (Options::read(&bytes[FIXED_LEN..HEADER_LEN]), HEADER_LEN)
        }
        other => return Err(BatchError::Version(other)),
    };
    let kind = Kind::from_code(bytes[9])?;
    options.validate(kind)?;
    let format = bytes[10];
    let width = u32_at(bytes, 12);
    let height = u32_at(bytes, 16);
    let count = u32_at(bytes, 20);
    if count > MAX_RECORDS {
        return Err(BatchError::TooManyRecords(count));
    }
    let mut records = Vec::with_capacity(count as usize);
    let mut position = body;
    for _ in 0..count {
        let len = u32::from_le_bytes(
            bytes
                .get(position..position + 4)
                .ok_or(BatchError::Truncated)?
                .try_into()
                .map_err(|_| BatchError::Truncated)?,
        );
        if len > MAX_RECORD_BYTES {
            return Err(BatchError::RecordTooLarge(len));
        }
        position += 4;
        let end = position + len as usize;
        records.push(bytes.get(position..end).ok_or(BatchError::Truncated)?);
        position = end;
    }
    if position != bytes.len() {
        return Err(BatchError::TrailingBytes);
    }
    Ok(Batch {
        header: Header {
            kind,
            format,
            width,
            height,
            count,
            options,
        },
        records,
    })
}

/// Écrit un lot (version 2) à partir de son en-tête et de ses enregistrements.
pub fn build(header: Header, records: &[Vec<u8>]) -> Result<Vec<u8>, BatchError> {
    header.options.validate(header.kind)?;
    if records.len() > MAX_RECORDS as usize {
        return Err(BatchError::TooManyRecords(records.len() as u32));
    }
    let mut out =
        Vec::with_capacity(HEADER_LEN + records.iter().map(|r| r.len() + 4).sum::<usize>());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(header.kind.code());
    out.push(header.format);
    out.push(0);
    out.extend_from_slice(&header.width.to_le_bytes());
    out.extend_from_slice(&header.height.to_le_bytes());
    out.extend_from_slice(&(records.len() as u32).to_le_bytes());
    header.options.write(&mut out);
    for record in records {
        let len = u32::try_from(record.len()).map_err(|_| BatchError::RecordTooLarge(u32::MAX))?;
        if len > MAX_RECORD_BYTES {
            return Err(BatchError::RecordTooLarge(len));
        }
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(record);
    }
    Ok(out)
}

/// Paramètres d'un fragment : la plage d'enregistrements `[start, end)`.
pub fn range_params(start: u32, end: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(8);
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&end.to_le_bytes());
    out
}

/// Lit une plage de [`range_params`], contrôlée contre le nombre d'enregistrements du lot.
pub fn parse_range(params: &[u8], count: u32) -> Result<(u32, u32), BatchError> {
    if params.len() != 8 {
        return Err(BatchError::BadRange);
    }
    let start = u32_at(params, 0);
    let end = u32_at(params, 4);
    if start > end || end > count {
        return Err(BatchError::BadRange);
    }
    Ok((start, end))
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    let mut word = [0u8; 4];
    word.copy_from_slice(&bytes[at..at + 4]);
    u32::from_le_bytes(word)
}

#[cfg(test)]
mod tests;
