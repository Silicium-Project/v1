//! Scene: built from the syntax tree for one animation frame.
//!
//! See the README of the V0 engine for the file format: rays per pixel and lighting mode (`render`),
//! a free camera (`look_at`, `direction`, `rotation`, depth of field), named materials (`phong`,
//! `diffuse`, `metal`, `glass`, `emissive`), spheres, planes, boxes, triangles, quads and meshes
//! with transforms, point and area lights, and keyframes.

mod build;
mod camera;
mod primitives;
mod resolution;
mod settings;
mod values;

pub use build::build;
pub use resolution::scene_resolution;

use crate::geometry::{Bvh, Shape};
use crate::math::Vec3;

/// How light is computed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Classic lighting (Phong highlights, mirror reflections, hard or soft
    /// shadows from `point` and `area` lights), plus glass and metal.
    Whitted,
    /// Monte-Carlo path tracing: indirect light, soft shadows, emissive objects.
    Path,
}

/// How colours above 1 are brought back to the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tonemap {
    Clamp,
    Reinhard,
    Aces,
}

/// The `render` block.
#[derive(Clone, Debug)]
pub struct Settings {
    pub samples: u32,
    pub mode: Mode,
    pub bounces: u32,
    pub exposure: f64,
    pub tonemap: Tonemap,
    /// Strength of the sky (background and, in path mode, its light).
    pub sky: f64,
    pub seed: u64,
    pub frame: f64,
    /// Largest contribution of a light reached after a diffuse bounce
    /// (path mode; removes "fireflies").
    pub clamp: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialKind {
    Phong,
    Diffuse,
    Metal,
    Glass,
    Emissive,
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub kind: MaterialKind,
    /// Colour in `0..=1` (written `0..=255` in scene files).
    pub color: Vec3,
    pub reflectivity: f64,
    pub specular: f64,
    pub shininess: f64,
    pub roughness: f64,
    pub ior: f64,
    /// Emitted radiance (emissive materials and area lights).
    pub emission: Vec3,
}

/// Point light (`intensity` 450 at 5 units is bright).
#[derive(Clone, Copy, Debug)]
pub struct PointLight {
    pub position: Vec3,
    pub intensity: f64,
    pub color: Vec3,
}

/// An object that emits light and is sampled for direct lighting.
#[derive(Clone, Copy, Debug)]
pub struct Emitter {
    /// Index in `Scene::shapes`.
    pub shape: usize,
    /// For `area` lights: their intensity, used by the Whitted mode.
    pub intensity: Option<f64>,
    pub color: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub origin: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub tan_half_fov: f64,
    pub aperture: f64,
    pub focus_distance: f64,
}

/// A scene ready to render (one frame).
#[derive(Clone, Debug)]
pub struct Scene {
    pub width: usize,
    pub height: usize,
    pub camera: Camera,
    pub settings: Settings,
    pub shapes: Vec<Shape>,
    pub shape_material: Vec<usize>,
    pub materials: Vec<Material>,
    pub planes: Vec<usize>,
    pub bvh: Bvh,
    pub points: Vec<PointLight>,
    pub emitters: Vec<Emitter>,
}

/// Scale of light intensities in path mode: a light of intensity 450 at 5
/// units lights a white surface about as much as in Whitted mode.
pub const LIGHT_UNIT: f64 = 1.0 / 12.0;
