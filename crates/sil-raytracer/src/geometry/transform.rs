//! Object transforms: scale, rotation, translation.

use crate::math::Vec3;
use crate::random::{cos, sin};

/// Scale, then rotation (degrees around x, then y, then z), then translation.
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    scale: Vec3,
    rotation: [Vec3; 3],
    translation: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            scale: Vec3::new(1.0, 1.0, 1.0),
            rotation: [
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            translation: Vec3::default(),
        }
    }
}

impl Transform {
    pub fn new(scale: Vec3, rotation_degrees: Vec3, translation: Vec3) -> Transform {
        let (ax, ay, az) = (
            rotation_degrees.x.to_radians(),
            rotation_degrees.y.to_radians(),
            rotation_degrees.z.to_radians(),
        );
        let (sx, cx, sy, cy, sz, cz) = (sin(ax), cos(ax), sin(ay), cos(ay), sin(az), cos(az));
        // Rows of Rz · Ry · Rx.
        let rotation = [
            Vec3::new(cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx),
            Vec3::new(sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx),
            Vec3::new(-sy, cy * sx, cy * cx),
        ];
        Transform {
            scale,
            rotation,
            translation,
        }
    }

    fn rotate(&self, vector: Vec3) -> Vec3 {
        Vec3::new(
            self.rotation[0].dot(vector),
            self.rotation[1].dot(vector),
            self.rotation[2].dot(vector),
        )
    }

    /// A point of the object in world coordinates.
    pub fn point(&self, point: Vec3) -> Vec3 {
        self.rotate(point.hadamard(self.scale)) + self.translation
    }

    /// An edge (a difference of points) in world coordinates.
    pub fn vector(&self, vector: Vec3) -> Vec3 {
        self.rotate(vector.hadamard(self.scale))
    }

    /// A surface normal in world coordinates.
    pub fn normal(&self, normal: Vec3) -> Vec3 {
        self.rotate(Vec3::new(
            normal.x / self.scale.x,
            normal.y / self.scale.y,
            normal.z / self.scale.z,
        ))
        .normalized()
    }

    /// Largest scale factor (spheres stay round).
    pub fn uniform_scale(&self) -> f64 {
        self.scale
            .x
            .abs()
            .max(self.scale.y.abs())
            .max(self.scale.z.abs())
    }
}
