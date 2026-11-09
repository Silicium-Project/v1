//! The `render` block: samples, lighting mode, tone mapping, animation frame.

use super::{Mode, Settings, Tonemap};
use crate::ENGINE_VERSION;
use crate::syntax::{self, BlockExt, Entry};

pub(super) fn settings(root: &[Entry], frame_override: Option<f64>) -> Result<Settings, String> {
    let empty: &[Entry] = &[];
    let render = root.block("render").unwrap_or(empty);
    let engine = render.number("engine", f64::from(ENGINE_VERSION), 0.0);
    if engine > f64::from(ENGINE_VERSION) {
        return Err(format!(
            "scene_requires_engine:{engine} (this engine is {ENGINE_VERSION})"
        ));
    }
    let mode = match render.word("mode").unwrap_or("whitted") {
        "whitted" => Mode::Whitted,
        "path" => Mode::Path,
        other => return Err(format!("unknown render mode `{other}`")),
    };
    let tonemap =
        match render
            .word("tonemap")
            .unwrap_or(if mode == Mode::Path { "aces" } else { "clamp" })
        {
            "clamp" => Tonemap::Clamp,
            "reinhard" => Tonemap::Reinhard,
            "aces" => Tonemap::Aces,
            other => return Err(format!("unknown tonemap `{other}`")),
        };
    let frame = frame_override
        // The last top-level `frame` wins: a frame sequence appends one.
        .or_else(|| {
            root.iter()
                .rev()
                .find(|entry| entry.key.as_deref() == Some("frame"))
                .and_then(|entry| syntax::evaluate(&entry.value, 0.0))
        })
        .unwrap_or_else(|| render.number("frame", 0.0, 0.0));
    Ok(Settings {
        samples: render.number("samples", 1.0, frame).clamp(1.0, 65_536.0) as u32,
        mode,
        bounces: render
            .number("bounces", if mode == Mode::Path { 8.0 } else { 3.0 }, frame)
            .clamp(0.0, 64.0) as u32,
        exposure: render.number("exposure", 1.0, frame).max(0.0),
        tonemap,
        sky: render.number("sky", 1.0, frame).max(0.0),
        seed: render.number("seed", 0.0, frame).max(0.0) as u64,
        frame,
        clamp: render.number("clamp", 2.0, frame).max(0.0),
    })
}
