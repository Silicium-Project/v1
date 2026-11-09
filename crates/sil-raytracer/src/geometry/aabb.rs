//! Axis-aligned bounding box.

use crate::math::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn point(point: Vec3) -> Aabb {
        Aabb {
            min: point,
            max: point,
        }
    }

    pub fn grow(&mut self, point: Vec3) {
        self.min = Vec3::new(
            self.min.x.min(point.x),
            self.min.y.min(point.y),
            self.min.z.min(point.z),
        );
        self.max = Vec3::new(
            self.max.x.max(point.x),
            self.max.y.max(point.y),
            self.max.z.max(point.z),
        );
    }

    pub fn merge(&mut self, other: &Aabb) {
        self.grow(other.min);
        self.grow(other.max);
    }

    pub fn centre(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// True when the ray enters the box before `limit`.
    pub fn hit(&self, origin: Vec3, inverse: Vec3, limit: f64) -> bool {
        let mut near = 0.0_f64;
        let mut far = limit;
        for axis in 0..3 {
            let a = (self.min.axis(axis) - origin.axis(axis)) * inverse.axis(axis);
            let b = (self.max.axis(axis) - origin.axis(axis)) * inverse.axis(axis);
            let (low, high) = if a <= b { (a, b) } else { (b, a) };
            // NaN (0 × ∞ on an axis-parallel ray at a slab edge) never shrinks the interval.
            if low > near {
                near = low;
            }
            if high < far {
                far = high;
            }
            if near > far {
                return false;
            }
        }
        true
    }
}
