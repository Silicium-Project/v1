//! Geometry: spheres, planes, triangles and parallelograms ("quads"), object
//! transforms, and the bounding-volume hierarchy (BVH) that keeps scenes with
//! many triangles fast.

mod aabb;
mod bvh;
mod shape;
#[cfg(test)]
mod tests;
mod transform;

pub use aabb::Aabb;
pub use bvh::Bvh;
pub use shape::Shape;
pub use transform::Transform;
