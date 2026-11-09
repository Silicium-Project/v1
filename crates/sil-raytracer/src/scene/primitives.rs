//! Primitives: the shapes a `primitives` block declares, with their materials.

use super::Material;
use super::values::{MATERIAL_KEYS, material, transform, vec3};
use crate::EPSILON;
use crate::geometry::Shape;
use crate::math::Vec3;
use crate::syntax::{BlockExt, Entry};
use std::collections::HashMap;

/// Shapes and materials of the scene being built.
pub(super) struct Builder {
    pub(super) frame: f64,
    pub(super) shapes: Vec<Shape>,
    pub(super) shape_material: Vec<usize>,
    pub(super) materials: Vec<Material>,
    pub(super) named: HashMap<String, usize>,
}

impl Builder {
    pub(super) fn new(frame: f64) -> Self {
        Builder {
            frame,
            shapes: Vec::new(),
            shape_material: Vec::new(),
            materials: Vec::new(),
            named: HashMap::new(),
        }
    }

    /// Material of an object: `material = name`, else its own keys.
    fn object_material(
        &mut self,
        block: &[Entry],
        default_reflectivity: f64,
    ) -> Result<usize, String> {
        if let Some(name) = block.word("material") {
            let base = *self.named.get(name).ok_or_else(|| {
                format!(
                    "line {}: unknown material `{name}`",
                    block.entry("material").map_or(0, |e| e.line)
                )
            })?;
            // Keys written on the object override the named material.
            if !MATERIAL_KEYS.iter().any(|key| block.entry(key).is_some()) {
                return Ok(base);
            }
        }
        let own = material(block, default_reflectivity, self.frame)?;
        self.materials.push(own);
        Ok(self.materials.len() - 1)
    }

    pub(super) fn add(&mut self, shape: Shape, material: usize) {
        self.shapes.push(shape);
        self.shape_material.push(material);
    }

    pub(super) fn primitive(
        &mut self,
        name: &str,
        block: &[Entry],
        line: usize,
    ) -> Result<(), String> {
        let frame = self.frame;
        let t = transform(block, frame);
        let n = |key: &str, default: f64| block.number(key, default, frame);
        match name {
            "sphere" => {
                let material = self.object_material(block, 0.32)?;
                let center = t.point(Vec3::new(n("x", 0.0), n("y", 0.0), n("z", 0.0)));
                let radius = (n("radius", 1.0) * t.uniform_scale()).max(EPSILON);
                self.add(Shape::Sphere { center, radius }, material);
            }
            "plane" => {
                let material = self.object_material(block, 0.12)?;
                let normal = match block.block("normal") {
                    Some(inner) => vec3(Some(inner), Vec3::new(0.0, 0.0, 1.0), frame),
                    None => Vec3::new(n("normal_x", 0.0), n("normal_y", 0.0), n("normal_z", 1.0)),
                };
                if normal.length() <= EPSILON {
                    return Err(format!("line {line}: plane normal is (0, 0, 0)"));
                }
                let point = t.point(Vec3::new(n("x", 0.0), n("y", 0.0), n("z", 0.0)));
                self.add(
                    Shape::Plane {
                        point,
                        normal: t.normal(normal.normalized()),
                    },
                    material,
                );
            }
            "box" => {
                let material = self.object_material(block, 0.12)?;
                let low = vec3(block.block("min"), Vec3::new(-0.5, -0.5, -0.5), frame);
                let high = vec3(block.block("max"), Vec3::new(0.5, 0.5, 0.5), frame);
                let corner = |i: usize| {
                    t.point(Vec3::new(
                        if i & 1 == 0 { low.x } else { high.x },
                        if i & 2 == 0 { low.y } else { high.y },
                        if i & 4 == 0 { low.z } else { high.z },
                    ))
                };
                // Faces as quads (corner, two edges), normals outward.
                for (a, b, c) in [
                    (0, 2, 1),
                    (4, 5, 6),
                    (0, 1, 4),
                    (2, 6, 3),
                    (0, 4, 2),
                    (1, 3, 5),
                ] {
                    let origin = corner(a);
                    self.add(
                        Shape::quad(origin, corner(b) - origin, corner(c) - origin),
                        material,
                    );
                }
            }
            "triangle" => {
                let material = self.object_material(block, 0.12)?;
                let a = t.point(vec3(block.block("a"), Vec3::default(), frame));
                let b = t.point(vec3(block.block("b"), Vec3::new(0.0, 1.0, 0.0), frame));
                let c = t.point(vec3(block.block("c"), Vec3::new(0.0, 0.0, 1.0), frame));
                self.add(Shape::triangle(a, b, c), material);
            }
            "quad" => {
                let material = self.object_material(block, 0.12)?;
                let corner = t.point(vec3(
                    block.block("corner"),
                    Vec3::new(0.0, -0.5, -0.5),
                    frame,
                ));
                let u = t.vector(vec3(block.block("u"), Vec3::new(0.0, 1.0, 0.0), frame));
                let v = t.vector(vec3(block.block("v"), Vec3::new(0.0, 0.0, 1.0), frame));
                self.add(Shape::quad(corner, u, v), material);
            }
            "mesh" => {
                let material = self.object_material(block, 0.12)?;
                let vertices = block
                    .block("vertices")
                    .map(|v| v.numbers())
                    .unwrap_or_default();
                let faces = block
                    .block("faces")
                    .map(|f| f.numbers())
                    .unwrap_or_default();
                if !vertices.len().is_multiple_of(3) || !faces.len().is_multiple_of(3) {
                    return Err(format!(
                        "line {line}: mesh vertices and faces must be groups of 3 numbers"
                    ));
                }
                let points: Vec<Vec3> = vertices
                    .chunks(3)
                    .map(|v| t.point(Vec3::new(v[0], v[1], v[2])))
                    .collect();
                for face in faces.chunks(3) {
                    let index = |value: f64| {
                        let i = value as usize;
                        (value >= 0.0 && value.fract() == 0.0 && i < points.len())
                            .then_some(i)
                            .ok_or_else(|| {
                                format!(
                                    "line {line}: mesh face uses vertex {value}, the mesh has {}",
                                    points.len()
                                )
                            })
                    };
                    let (a, b, c) = (index(face[0])?, index(face[1])?, index(face[2])?);
                    self.add(Shape::triangle(points[a], points[b], points[c]), material);
                }
            }
            _ => {}
        }
        Ok(())
    }
}
