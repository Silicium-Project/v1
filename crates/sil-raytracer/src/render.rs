//! Rendering one band of rows of a scene: the scene is read from its text, then the rows
//! `[y0, y1)` are rendered in order, pixel by pixel. Output: RGB24, row by row, no padding.

use crate::integrator::pixel;
use crate::scene::build;
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

/// RGB24 pixels of the rows `[y0, y1)` of a `width` × `height` image of the scene `text`.
/// The same scene and rows give the same bytes on every machine.
pub fn render_rows(
    text: &str,
    width: u32,
    height: u32,
    y0: u32,
    y1: u32,
) -> Result<Vec<u8>, RenderError> {
    let scene = build(text, None).map_err(RenderError::Scene)?;
    if scene.width != width as usize || scene.height != height as usize {
        return Err(RenderError::Size {
            scene: (scene.width, scene.height),
            asked: (width, height),
        });
    }
    if y0 >= y1 || y1 > height {
        return Err(RenderError::Band { y0, y1, height });
    }
    let row_bytes = width as usize * 3;
    let mut out = Vec::with_capacity(row_bytes * (y1 - y0) as usize);
    for y in y0..y1 {
        for x in 0..width {
            out.extend_from_slice(&pixel(&scene, x as usize, y as usize));
        }
    }
    Ok(out)
}
