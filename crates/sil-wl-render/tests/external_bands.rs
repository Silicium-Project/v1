//! Plusieurs bandes en un appel au moteur : mêmes octets que un appel par bande, et un ancien
//! moteur (code 2 sur `bands`) retombe sur un appel par bande.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sil_wl_render::{BlockRenderer, ExternalRenderer, reference_pixels};
use std::{path::PathBuf, time::Duration};

const ENGINE: &str = env!("CARGO_BIN_EXE_sil-render-reference");

fn engine() -> ExternalRenderer {
    ExternalRenderer::new(
        PathBuf::from(ENGINE),
        std::env::temp_dir(),
        Duration::from_secs(30),
    )
}

fn scene() -> Vec<u8> {
    (0..64u8).map(|i| i.wrapping_mul(29) ^ 0x5A).collect()
}

#[test]
fn several_bands_in_one_call_are_the_bands_concatenated() {
    let bands = [(8, 16), (0, 4), (20, 24)];
    let together = engine()
        .render_bands(&scene(), 16, 24, &bands)
        .expect("bandes");
    let mut expected = Vec::new();
    for (y0, y1) in bands {
        expected.extend(reference_pixels(&scene(), 16, y0, y1));
    }
    assert_eq!(together, expected);
    assert_eq!(together.len(), 16 * (8 + 4 + 4) * 3);
}

#[test]
fn one_band_uses_the_single_band_protocol() {
    let bands = [(8, 16)];
    let rendered = engine()
        .render_bands(&scene(), 16, 24, &bands)
        .expect("une bande");
    assert_eq!(rendered, reference_pixels(&scene(), 16, 8, 16));
}
