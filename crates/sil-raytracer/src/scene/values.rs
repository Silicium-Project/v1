//! Reading values from blocks: vectors, colours, transforms and materials.

use super::{Material, MaterialKind};
use crate::geometry::Transform;
use crate::math::Vec3;
use crate::syntax::{self, BlockExt, Entry, Value};

pub(super) fn vec3(block: Option<&[Entry]>, default: Vec3, frame: f64) -> Vec3 {
    let Some(block) = block else {
        return default;
    };
    let bare = block.numbers();
    if bare.len() == 3 {
        return Vec3::new(bare[0], bare[1], bare[2]);
    }
    Vec3::new(
        block.number("x", default.x, frame),
        block.number("y", default.y, frame),
        block.number("z", default.z, frame),
    )
}

pub(super) fn color(block: &[Entry], default: f64, frame: f64) -> Vec3 {
    let source = block.block("color").unwrap_or(block);
    let bare = block
        .block("color")
        .map(|c| c.numbers())
        .unwrap_or_default();
    if bare.len() == 3 {
        return Vec3::new(bare[0], bare[1], bare[2]) / 255.0;
    }
    Vec3::new(
        source.number("r", block.number("r", default, frame), frame),
        source.number("g", block.number("g", default, frame), frame),
        source.number("b", block.number("b", default, frame), frame),
    ) / 255.0
}

pub(super) fn transform(block: &[Entry], frame: f64) -> Transform {
    let scale = match block.entry("scale").map(|entry| &entry.value) {
        Some(Value::Block(inner)) if syntax::keyframes(inner).is_none() => {
            vec3(Some(inner.as_slice()), Vec3::new(1.0, 1.0, 1.0), frame)
        }
        Some(value) => {
            let s = syntax::evaluate(value, frame).unwrap_or(1.0);
            Vec3::new(s, s, s)
        }
        None => Vec3::new(1.0, 1.0, 1.0),
    };
    Transform::new(
        scale,
        vec3(block.block("rotate"), Vec3::default(), frame),
        vec3(block.block("translate"), Vec3::default(), frame),
    )
}

fn material_kind(word: &str) -> Result<MaterialKind, String> {
    Ok(match word {
        "phong" => MaterialKind::Phong,
        "diffuse" => MaterialKind::Diffuse,
        "metal" => MaterialKind::Metal,
        "glass" => MaterialKind::Glass,
        "emissive" => MaterialKind::Emissive,
        other => return Err(format!("unknown material type `{other}`")),
    })
}

/// A material from the keys of `block` (a `material` block or an object).
pub(super) fn material(
    block: &[Entry],
    default_reflectivity: f64,
    frame: f64,
) -> Result<Material, String> {
    let kind = block
        .word("type")
        .map(material_kind)
        .transpose()?
        .unwrap_or(MaterialKind::Phong);
    let color = color(block, 200.0, frame);
    let (reflectivity, specular) = match kind {
        MaterialKind::Diffuse => (0.0, 0.0),
        _ => (
            block
                .number(
                    "reflectivity",
                    block.number("reflection", default_reflectivity, frame),
                    frame,
                )
                .clamp(0.0, 0.95),
            block.number("specular", 0.65, frame).clamp(0.0, 2.0),
        ),
    };
    let strength = block.number("strength", 1.0, frame).max(0.0);
    Ok(Material {
        kind,
        color,
        reflectivity,
        specular,
        shininess: block.number("shininess", 72.0, frame).clamp(1.0, 512.0),
        roughness: block.number("roughness", 0.0, frame).clamp(0.0, 1.0),
        ior: block.number("ior", 1.5, frame).clamp(1.0, 3.0),
        emission: if kind == MaterialKind::Emissive {
            color.hadamard(color) * strength
        } else {
            Vec3::default()
        },
    })
}

/// Keys that make an object carry its own material, even if it names one.
pub(super) const MATERIAL_KEYS: [&str; 12] = [
    "type",
    "color",
    "r",
    "g",
    "b",
    "reflection",
    "reflectivity",
    "specular",
    "shininess",
    "roughness",
    "ior",
    "strength",
];
