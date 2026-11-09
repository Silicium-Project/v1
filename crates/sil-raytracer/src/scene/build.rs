//! Building a scene from its text: the blocks are read in the order materials, primitives, lights.

use super::camera::camera;
use super::primitives::Builder;
use super::settings::settings;
use super::values::{color, material};
use super::{Emitter, LIGHT_UNIT, Material, MaterialKind, Mode, PointLight, Scene};
use crate::EPSILON;
use crate::geometry::{Bvh, Shape};
use crate::math::Vec3;
use crate::syntax::{self, BlockExt, Entry};

/// Builds the scene of `text` at `frame` (the scene's own frame when `None`).
pub fn build(text: &str, frame_override: Option<f64>) -> Result<Scene, String> {
    let root = syntax::parse(text).map_err(|error| error.to_string())?;
    let settings = settings(&root, frame_override)?;
    let frame = settings.frame;
    let empty: Vec<Entry> = Vec::new();
    let camera_block = root.block("camera").unwrap_or(&empty);
    let resolution = camera_block.block("resolution").unwrap_or(&empty);
    let width = resolution.number("width", 320.0, 0.0).max(1.0) as usize;
    let height = resolution.number("height", 200.0, 0.0).max(1.0) as usize;

    let mut builder = Builder::new(frame);
    if let Some(materials) = root.block("materials") {
        for entry in materials.all("material") {
            for block in syntax::items(&entry.value) {
                let name = block
                    .word("name")
                    .ok_or_else(|| format!("line {}: material without `name`", entry.line))?;
                let parsed = material(block, 0.32, frame)?;
                builder.materials.push(parsed);
                let index = builder.materials.len() - 1;
                builder.named.insert(name.to_string(), index);
            }
        }
    }
    if let Some(primitives) = root.block("primitives") {
        for entry in primitives {
            let Some(name) = entry.key.as_deref() else {
                continue;
            };
            for block in syntax::items(&entry.value) {
                builder.primitive(name, block, entry.line)?;
            }
        }
    }
    let mut points = Vec::new();
    let mut emitters = Vec::new();
    if let Some(lights) = root.block("lights") {
        let items = lights.iter().flat_map(|entry| {
            syntax::items(&entry.value)
                .into_iter()
                .map(move |block| (entry.key.as_deref(), block))
        });
        for (name, block) in items {
            let Some(name) = name else {
                continue;
            };
            let n = |key: &str, default: f64| block.number(key, default, frame);
            let position = Vec3::new(n("x", 4.0), n("y", 0.0), n("z", 5.0));
            let intensity = n("intensity", 450.0).max(0.0);
            let tint = color(block, 255.0, frame);
            match name {
                "point" => points.push(PointLight {
                    position,
                    intensity,
                    color: tint,
                }),
                "area" => {
                    let radius = n("radius", 0.5).max(EPSILON);
                    builder.materials.push(Material {
                        kind: MaterialKind::Emissive,
                        color: tint,
                        reflectivity: 0.0,
                        specular: 0.0,
                        shininess: 1.0,
                        roughness: 0.0,
                        ior: 1.0,
                        emission: tint * (intensity * LIGHT_UNIT / (radius * radius)),
                    });
                    let material = builder.materials.len() - 1;
                    builder.add(
                        Shape::Sphere {
                            center: position,
                            radius,
                        },
                        material,
                    );
                    emitters.push(Emitter {
                        shape: builder.shapes.len() - 1,
                        intensity: Some(intensity),
                        color: tint,
                    });
                }
                _ => {}
            }
        }
    }
    for (index, shape) in builder.shapes.iter().enumerate() {
        let material = &builder.materials[builder.shape_material[index]];
        let sampled = matches!(
            shape,
            Shape::Sphere { .. } | Shape::Quad { .. } | Shape::Triangle { .. }
        );
        if material.kind == MaterialKind::Emissive
            && sampled
            && !emitters.iter().any(|e: &Emitter| e.shape == index)
        {
            emitters.push(Emitter {
                shape: index,
                intensity: None,
                color: material.emission,
            });
        }
    }
    if settings.mode == Mode::Whitted
        && points.is_empty()
        && emitters.iter().all(|e| e.intensity.is_none())
    {
        // Default light, as in the first versions of the engine.
        points.push(PointLight {
            position: Vec3::new(4.0, 0.0, 5.0),
            intensity: 450.0,
            color: Vec3::new(1.0, 1.0, 1.0),
        });
    }
    let planes: Vec<usize> = (0..builder.shapes.len())
        .filter(|i| matches!(builder.shapes[*i], Shape::Plane { .. }))
        .collect();
    let finite: Vec<usize> = (0..builder.shapes.len())
        .filter(|i| !planes.contains(i))
        .collect();
    let bvh = Bvh::build(&builder.shapes, finite);
    Ok(Scene {
        width,
        height,
        camera: camera(camera_block, frame),
        settings,
        shapes: builder.shapes,
        shape_material: builder.shape_material,
        materials: builder.materials,
        planes,
        bvh,
        points,
        emitters,
    })
}
