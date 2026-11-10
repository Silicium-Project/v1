use crate::transform::apply;
use image::{DynamicImage, ImageFormat, ImageReader, Limits, codecs::jpeg::JpegEncoder};
use sil_batch::Header;
use sil_workload_api::WorkloadError;
use std::io::Cursor;

/// Format de sortie : PNG (sans perte) ou JPEG (qualité réglable, défaut de l'encodeur si 0).
pub const FORMAT_PNG: u8 = 0;
pub const FORMAT_JPEG: u8 = 1;

/// Mémoire maximale allouée pour décoder une image (protège des images « bombes »).
const MAX_DECODE_BYTES: u64 = 256 * 1024 * 1024;

/// Décode une image, lui applique les options de l'en-tête, et la réencode au format demandé.
/// Même entrée, mêmes octets : le décodage, les transformations et l'encodeur sont déterministes.
pub fn process(record: &[u8], header: Header) -> Result<Vec<u8>, WorkloadError> {
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    let mut reader = ImageReader::new(Cursor::new(record))
        .with_guessed_format()
        .map_err(|e| WorkloadError::BadInput(format!("image illisible : {e}")))?;
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|e| WorkloadError::BadInput(format!("image non décodable : {e}")))?;
    let image = apply(image, header);
    encode(&image, header.format, header.options.quality)
}

fn encode(image: &DynamicImage, format: u8, quality: u8) -> Result<Vec<u8>, WorkloadError> {
    let mut out = Cursor::new(Vec::new());
    let written = match format {
        FORMAT_PNG => image.write_to(&mut out, ImageFormat::Png),
        FORMAT_JPEG => {
            let rgb = DynamicImage::ImageRgb8(image.to_rgb8());
            if quality == 0 {
                rgb.write_with_encoder(JpegEncoder::new(&mut out))
            } else {
                rgb.write_with_encoder(JpegEncoder::new_with_quality(&mut out, quality))
            }
        }
        other => {
            return Err(WorkloadError::BadInput(format!(
                "format de sortie inconnu : {other}"
            )));
        }
    };
    written.map_err(|e| WorkloadError::Execution(format!("encodage impossible : {e}")))?;
    Ok(out.into_inner())
}
