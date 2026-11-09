//! The camera: position, orientation, field of view and depth of field.

use super::Camera;
use super::values::vec3;
use crate::EPSILON;
use crate::geometry::Transform;
use crate::math::Vec3;
use crate::random::tan;
use crate::syntax::{BlockExt, Entry};

pub(super) fn camera(block: &[Entry], frame: f64) -> Camera {
    let n = |key: &str, default: f64| block.number(key, default, frame);
    let position = match block.block("position") {
        Some(inner) => vec3(Some(inner), Vec3::default(), frame),
        None => Vec3::new(n("x", 0.0), n("y", 0.0), n("z", 0.0)),
    };
    let look_at = block
        .block("look_at")
        .map(|inner| vec3(Some(inner), Vec3::new(1.0, 0.0, 0.0), frame));
    let up_hint = vec3(block.block("up"), Vec3::new(0.0, 0.0, 1.0), frame).normalized();
    let mut up_hint = if up_hint.length() <= EPSILON {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        up_hint
    };
    let mut forward = if let Some(target) = look_at {
        target - position
    } else if let Some(direction) = block.block("direction") {
        vec3(Some(direction), Vec3::new(1.0, 0.0, 0.0), frame)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    }
    .normalized();
    if forward.length() <= EPSILON {
        forward = Vec3::new(1.0, 0.0, 0.0);
    }
    if look_at.is_none()
        && let Some(rotation) = block.block("rotation")
    {
        let turn = Transform::new(
            Vec3::new(1.0, 1.0, 1.0),
            vec3(Some(rotation), Vec3::default(), frame),
            Vec3::default(),
        );
        forward = turn.vector(forward).normalized();
        up_hint = turn.vector(up_hint).normalized();
    }
    if forward.cross(up_hint).length() <= 1e-9 {
        up_hint = if forward.z.abs() > 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
    }
    let right = forward.cross(up_hint).normalized();
    let up = right.cross(forward).normalized();
    let fov = n("fieldOfView", 55.0).clamp(1.0, 179.0);
    let aperture = n("aperture", 0.0).max(0.0);
    let focus_distance = n(
        "focus_distance",
        look_at
            .map(|target| (target - position).length())
            .unwrap_or(5.0),
    )
    .max(EPSILON);
    Camera {
        origin: position,
        forward,
        right,
        up,
        tan_half_fov: tan(fov.to_radians() * 0.5),
        aperture,
        focus_distance,
    }
}
