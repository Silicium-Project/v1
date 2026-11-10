//! Rendering bands of rows of a scene: the scene is read from its text once, then the rows are
//! rendered pixel by pixel, split across worker threads. Output: RGB24, row by row, no padding.
//! Each pixel depends only on its coordinates and the scene, so the bytes do not depend on the
//! number of threads.

use crate::integrator::pixel;
use crate::scene::{Scene, build};
use std::fmt;

/// Why a band could not be rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// The scene text is invalid (syntax, unknown key or value, engine too new).
    Scene(String),
    /// The image size asked by the job differs from the resolution of the scene.
    Size {
        scene: (usize, usize),
        asked: (u32, u32),
    },
    /// The rows are not inside the image.
    Band { y0: u32, y1: u32, height: u32 },
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::Scene(message) => write!(f, "scene: {message}"),
            RenderError::Size { scene, asked } => write!(
                f,
                "scene resolution {}x{} differs from the job's {}x{}",
                scene.0, scene.1, asked.0, asked.1
            ),
            RenderError::Band { y0, y1, height } => {
                write!(f, "rows {y0}..{y1} are outside the {height}-row image")
            }
        }
    }
}

/// RGB24 pixels of the rows `[y0, y1)` of a `width` x `height` image of the scene `text`,
/// on as many threads as the machine has cores. The same scene and rows give the same bytes
/// on every machine.
pub fn render_rows(
    text: &str,
    width: u32,
    height: u32,
    y0: u32,
    y1: u32,
) -> Result<Vec<u8>, RenderError> {
    render_bands(text, width, height, &[(y0, y1)], default_threads())
}

/// As [`render_rows`], with `threads` workers (at least one). The bytes do not depend on
/// `threads`.
pub fn render_rows_threads(
    text: &str,
    width: u32,
    height: u32,
    y0: u32,
    y1: u32,
    threads: usize,
) -> Result<Vec<u8>, RenderError> {
    render_bands(text, width, height, &[(y0, y1)], threads)
}

/// The bands `[y0, y1)` one after another in `bands`, concatenated in the same order. The scene
/// is read once for all of them, and the rows are shared among `threads` workers.
pub fn render_bands(
    text: &str,
    width: u32,
    height: u32,
    bands: &[(u32, u32)],
    threads: usize,
) -> Result<Vec<u8>, RenderError> {
    let scene = build(text, None).map_err(RenderError::Scene)?;
    if scene.width != width as usize || scene.height != height as usize {
        return Err(RenderError::Size {
            scene: (scene.width, scene.height),
            asked: (width, height),
        });
    }
    for &(y0, y1) in bands {
        if y0 >= y1 || y1 > height {
            return Err(RenderError::Band { y0, y1, height });
        }
    }
    Ok(render_rows_of(&scene, width, bands, threads))
}

/// Workers for one call: the machine's cores on a native target. WebAssembly has no threads
/// here, so the browser build renders on one.
fn default_threads() -> usize {
    #[cfg(target_family = "wasm")]
    {
        1
    }
    #[cfg(not(target_family = "wasm"))]
    {
        std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
    }
}

/// Renders every row of `bands`, in order, into one buffer. Each row is a slot of the buffer
/// with its row number; the slots are split into consecutive parts, one per worker, so no
/// locking is needed.
fn render_rows_of(scene: &Scene, width: u32, bands: &[(u32, u32)], threads: usize) -> Vec<u8> {
    let row_bytes = width as usize * 3;
    let rows: usize = bands.iter().map(|(y0, y1)| (y1 - y0) as usize).sum();
    let mut out = vec![0u8; row_bytes * rows];
    let mut slots: Vec<(u32, &mut [u8])> = bands
        .iter()
        .flat_map(|&(y0, y1)| y0..y1)
        .zip(out.chunks_mut(row_bytes))
        .collect();
    let workers = threads.clamp(1, rows.max(1));
    let per_worker = slots.len().div_ceil(workers).max(1);
    #[cfg(not(target_family = "wasm"))]
    std::thread::scope(|spawner| {
        for part in slots.chunks_mut(per_worker) {
            spawner.spawn(move || fill_part(scene, width, part));
        }
    });
    #[cfg(target_family = "wasm")]
    for part in slots.chunks_mut(per_worker) {
        fill_part(scene, width, part);
    }
    out
}

/// Writes the pixels of each row of `part` (row number, RGB24 slot).
fn fill_part(scene: &Scene, width: u32, part: &mut [(u32, &mut [u8])]) {
    for (y, row) in part.iter_mut() {
        for (x, pixel_bytes) in row.chunks_mut(3).enumerate().take(width as usize) {
            pixel_bytes.copy_from_slice(&pixel(scene, x, *y as usize));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENE: &str = include_str!("../scenes/silicium-test-scene.cfg");

    #[test]
    fn threads_do_not_change_the_pixels() {
        let single = render_rows_threads(SCENE, 320, 200, 40, 52, 1).expect("one thread");
        let many = render_rows_threads(SCENE, 320, 200, 40, 52, 5).expect("five threads");
        assert_eq!(single.len(), 320 * 12 * 3);
        assert_eq!(single, many);
    }

    #[test]
    fn a_band_is_the_same_as_its_rows_in_a_full_render() {
        let band = render_rows_threads(SCENE, 320, 200, 10, 14, 3).expect("band");
        let rows = render_rows_threads(SCENE, 320, 200, 0, 14, 1).expect("rows");
        assert_eq!(band, rows[10 * 320 * 3..14 * 320 * 3]);
    }

    #[test]
    fn several_bands_are_the_bands_concatenated_in_order() {
        let bands = [(30, 34), (4, 6)];
        let together = render_bands(SCENE, 320, 200, &bands, 4).expect("bands");
        let mut apart = render_rows_threads(SCENE, 320, 200, 30, 34, 1).expect("first");
        apart.extend(render_rows_threads(SCENE, 320, 200, 4, 6, 1).expect("second"));
        assert_eq!(together, apart);
    }
}
