//! Shapes in world coordinates: ray intersection, bounds and area.

use super::Aabb;
use crate::{EPSILON, math::Vec3};

/// Shape of an object, in world coordinates.
#[derive(Clone, Debug)]
pub enum Shape {
    Sphere {
        center: Vec3,
        radius: f64,
    },
    Plane {
        point: Vec3,
        normal: Vec3,
    },
    Triangle {
        a: Vec3,
        edge1: Vec3,
        edge2: Vec3,
        normal: Vec3,
    },
    /// Parallelogram `corner + s·u + t·v`, `s, t` in `[0, 1]`.
    Quad {
        corner: Vec3,
        u: Vec3,
        v: Vec3,
        normal: Vec3,
    },
}

impl Shape {
    pub fn triangle(a: Vec3, b: Vec3, c: Vec3) -> Shape {
        let (edge1, edge2) = (b - a, c - a);
        Shape::Triangle {
            a,
            edge1,
            edge2,
            normal: edge1.cross(edge2).normalized(),
        }
    }

    pub fn quad(corner: Vec3, u: Vec3, v: Vec3) -> Shape {
        Shape::Quad {
            corner,
            u,
            v,
            normal: u.cross(v).normalized(),
        }
    }

    /// Distance and outward normal of the first hit after `EPSILON`.
    pub fn intersect(&self, origin: Vec3, direction: Vec3) -> Option<(f64, Vec3)> {
        match *self {
            Shape::Sphere { center, radius } => {
                let offset = origin - center;
                let half_b = offset.dot(direction);
                let c = offset.dot(offset) - radius * radius;
                let discriminant = half_b * half_b - c;
                if discriminant < 0.0 {
                    return None;
                }
                let root = discriminant.sqrt();
                let distance = [-half_b - root, -half_b + root]
                    .into_iter()
                    .find(|t| *t > EPSILON)?;
                let point = origin + direction * distance;
                Some((distance, (point - center) / radius))
            }
            Shape::Plane { point, normal } => {
                let denominator = normal.dot(direction);
                if denominator.abs() < 1e-12 {
                    return None;
                }
                let distance = (point - origin).dot(normal) / denominator;
                (distance > EPSILON).then_some((distance, normal))
            }
            Shape::Triangle {
                a,
                edge1,
                edge2,
                normal,
            } => {
                let p = direction.cross(edge2);
                let determinant = edge1.dot(p);
                if determinant.abs() < 1e-12 {
                    return None;
                }
                let inverse = 1.0 / determinant;
                let s = origin - a;
                let u = s.dot(p) * inverse;
                if !(0.0..=1.0).contains(&u) {
                    return None;
                }
                let q = s.cross(edge1);
                let v = direction.dot(q) * inverse;
                if v < 0.0 || u + v > 1.0 {
                    return None;
                }
                let distance = edge2.dot(q) * inverse;
                (distance > EPSILON).then_some((distance, normal))
            }
            Shape::Quad {
                corner,
                u,
                v,
                normal,
            } => {
                let denominator = normal.dot(direction);
                if denominator.abs() < 1e-12 {
                    return None;
                }
                let distance = (corner - origin).dot(normal) / denominator;
                if distance <= EPSILON {
                    return None;
                }
                let relative = origin + direction * distance - corner;
                let n = u.cross(v);
                let w = n / n.dot(n);
                let alpha = w.dot(relative.cross(v));
                let beta = w.dot(u.cross(relative));
                ((0.0..=1.0).contains(&alpha) && (0.0..=1.0).contains(&beta))
                    .then_some((distance, normal))
            }
        }
    }

    /// Bounding box, `None` for an infinite plane.
    pub fn bounds(&self) -> Option<Aabb> {
        let pad = Vec3::new(1e-6, 1e-6, 1e-6);
        match *self {
            Shape::Sphere { center, radius } => {
                let r = Vec3::new(radius, radius, radius);
                Some(Aabb {
                    min: center - r,
                    max: center + r,
                })
            }
            Shape::Plane { .. } => None,
            Shape::Triangle {
                a, edge1, edge2, ..
            } => {
                let mut bounds = Aabb::point(a);
                bounds.grow(a + edge1);
                bounds.grow(a + edge2);
                Some(Aabb {
                    min: bounds.min - pad,
                    max: bounds.max + pad,
                })
            }
            Shape::Quad { corner, u, v, .. } => {
                let mut bounds = Aabb::point(corner);
                for point in [corner + u, corner + v, corner + u + v] {
                    bounds.grow(point);
                }
                Some(Aabb {
                    min: bounds.min - pad,
                    max: bounds.max + pad,
                })
            }
        }
    }

    /// Surface area (spheres, triangles, quads).
    pub fn area(&self) -> f64 {
        match *self {
            Shape::Sphere { radius, .. } => 4.0 * std::f64::consts::PI * radius * radius,
            Shape::Plane { .. } => f64::INFINITY,
            Shape::Triangle { edge1, edge2, .. } => 0.5 * edge1.cross(edge2).length(),
            Shape::Quad { u, v, .. } => u.cross(v).length(),
        }
    }
}
