//! Light: transmittance, sky, refraction and reflectance, and the light reaching a point.

use super::{Hit, material_of, nearest};
use crate::EPSILON;
use crate::geometry::Shape;
use crate::math::Vec3;
use crate::random::{Rng, pow};
use crate::scene::{LIGHT_UNIT, MaterialKind, Scene};
use std::f64::consts::PI;

/// Light let through between `origin` and `distance` along `direction`:
/// black behind an opaque object, tinted behind glass. `light` (the shape
/// of the light being sampled) never hides itself.
pub(super) fn transmittance(
    scene: &Scene,
    mut origin: Vec3,
    direction: Vec3,
    mut distance: f64,
    light: Option<usize>,
) -> Vec3 {
    let mut through = Vec3::new(1.0, 1.0, 1.0);
    for _ in 0..8 {
        let Some(hit) = nearest(scene, origin, direction, distance - EPSILON) else {
            return through;
        };
        if Some(hit.shape) == light {
            return through;
        }
        let material = material_of(scene, &hit);
        if material.kind != MaterialKind::Glass {
            return Vec3::default();
        }
        through = through.hadamard(material.color) * 0.92;
        origin = hit.point + direction * (EPSILON * 4.0);
        distance -= hit.distance;
    }
    Vec3::default()
}

pub(super) fn background(scene: &Scene, direction: Vec3) -> Vec3 {
    let amount = (direction.z + 1.0) * 0.5;
    (Vec3::new(0.04, 0.07, 0.13) * (1.0 - amount) + Vec3::new(0.34, 0.55, 0.9) * amount)
        * scene.settings.sky
}

pub(super) fn refract(direction: Vec3, normal: Vec3, eta: f64) -> Option<Vec3> {
    let cos_i = (-direction).dot(normal).min(1.0);
    let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
    (k >= 0.0).then(|| (direction * eta + normal * (eta * cos_i - k.sqrt())).normalized())
}

/// Schlick's approximation of the Fresnel reflectance.
pub(super) fn fresnel(cos_i: f64, eta: f64) -> f64 {
    let ratio = (1.0 - eta) / (1.0 + eta);
    let r0 = ratio * ratio;
    let m = 1.0 - cos_i;
    r0 + (1.0 - r0) * (m * m * m * m * m)
}

/// Mirror direction blurred by `roughness` (0 = perfect mirror).
pub(super) fn glossy(reflected: Vec3, roughness: f64, rng: &mut Rng) -> Vec3 {
    if roughness <= 0.0 {
        return reflected;
    }
    (reflected + rng.in_sphere() * roughness).normalized()
}

/// A light seen from a shaded point (Whitted mode): direction, distance,
/// intensity and colour.
type SeenLight = (Vec3, f64, f64, Vec3, Option<usize>);

fn whitted_lights(scene: &Scene, point: Vec3, rng: &mut Rng) -> Vec<SeenLight> {
    let mut lights: Vec<SeenLight> = scene
        .points
        .iter()
        .map(|light| {
            let displacement = light.position - point;
            (
                displacement,
                displacement.dot(displacement),
                light.intensity,
                light.color,
                None,
            )
        })
        .collect();
    for emitter in &scene.emitters {
        let (Some(intensity), Shape::Sphere { center, radius }) =
            (emitter.intensity, &scene.shapes[emitter.shape])
        else {
            continue;
        };
        // A random point of the half of the light facing the point: soft
        // shadows over the samples.
        let mut offset = rng.on_sphere();
        if offset.dot(point - *center) < 0.0 {
            offset = -offset;
        }
        let displacement = *center + offset * *radius - point;
        lights.push((
            displacement,
            (*center - point).dot(*center - point),
            intensity,
            emitter.color,
            Some(emitter.shape),
        ));
    }
    lights
}

/// Classic (Phong) lighting of a point: diffuse light and highlights.
pub(super) fn phong_light(
    scene: &Scene,
    hit: &Hit,
    view: Vec3,
    shininess: f64,
    specular: f64,
    rng: &mut Rng,
) -> (Vec3, Vec3) {
    let mut diffuse = Vec3::new(0.09, 0.09, 0.11);
    let mut highlight = Vec3::default();
    for (displacement, centre_distance_squared, intensity, color, shape) in
        whitted_lights(scene, hit.point, rng)
    {
        let direction = displacement / displacement.length().max(EPSILON);
        let through = transmittance(
            scene,
            hit.point + hit.normal * (EPSILON * 4.0),
            direction,
            displacement.length(),
            shape,
        );
        if through.max_component() <= 0.0 {
            continue;
        }
        let attenuation = (intensity / centre_distance_squared.max(1.0)).min(1.5);
        let lambert = hit.normal.dot(direction).max(0.0);
        diffuse = diffuse + color.hadamard(through) * (lambert * attenuation);
        let reflected = (-direction).reflect(hit.normal).normalized();
        let shine = pow(reflected.dot(view).max(0.0), shininess) * specular * attenuation;
        highlight = highlight + color.hadamard(through) * shine;
    }
    (diffuse, highlight)
}

/// Irradiance at a point of normal `normal` from the point lights and one
/// sample of each emitting object (path mode).
pub(super) fn direct_light(scene: &Scene, point: Vec3, normal: Vec3, rng: &mut Rng) -> Vec3 {
    let origin = point + normal * (EPSILON * 4.0);
    let mut irradiance = Vec3::default();
    for light in &scene.points {
        let displacement = light.position - point;
        let distance_squared = displacement.dot(displacement).max(1e-6);
        let distance = distance_squared.sqrt();
        let direction = displacement / distance;
        let cosine = normal.dot(direction);
        if cosine <= 0.0 {
            continue;
        }
        let through = transmittance(scene, origin, direction, distance, None);
        irradiance = irradiance
            + light.color.hadamard(through)
                * (PI * light.intensity * LIGHT_UNIT * cosine / distance_squared);
    }
    for emitter in &scene.emitters {
        let shape = &scene.shapes[emitter.shape];
        let radiance = scene.materials[scene.shape_material[emitter.shape]].emission;
        let (direction, distance, inverse_pdf) = match *shape {
            Shape::Sphere { center, radius } => {
                let axis = center - point;
                let distance_squared = axis.dot(axis);
                if distance_squared <= radius * radius {
                    continue;
                }
                let cos_max = (1.0 - radius * radius / distance_squared).max(0.0).sqrt();
                let direction = rng.in_cone(axis.normalized(), cos_max);
                let Some((distance, _)) = shape.intersect(origin, direction) else {
                    continue;
                };
                (direction, distance, 2.0 * PI * (1.0 - cos_max))
            }
            Shape::Quad {
                corner,
                u,
                v,
                normal: light_normal,
            } => {
                let target = corner + u * rng.uniform() + v * rng.uniform();
                let (direction, distance, cos_light) = towards(point, target, light_normal);
                if cos_light <= 1e-9 {
                    continue;
                }
                (
                    direction,
                    distance,
                    shape.area() * cos_light / (distance * distance),
                )
            }
            Shape::Triangle {
                a,
                edge1,
                edge2,
                normal: light_normal,
            } => {
                let (mut s, mut t) = (rng.uniform(), rng.uniform());
                if s + t > 1.0 {
                    (s, t) = (1.0 - s, 1.0 - t);
                }
                let target = a + edge1 * s + edge2 * t;
                let (direction, distance, cos_light) = towards(point, target, light_normal);
                if cos_light <= 1e-9 {
                    continue;
                }
                (
                    direction,
                    distance,
                    shape.area() * cos_light / (distance * distance),
                )
            }
            Shape::Plane { .. } => continue,
        };
        let cosine = normal.dot(direction);
        if cosine <= 0.0 {
            continue;
        }
        let through = transmittance(scene, origin, direction, distance, Some(emitter.shape));
        irradiance = irradiance + radiance.hadamard(through) * (cosine * inverse_pdf);
    }
    irradiance
}

/// Direction and distance from `point` to `target`, and the cosine at the
/// (two-sided) light surface.
fn towards(point: Vec3, target: Vec3, light_normal: Vec3) -> (Vec3, f64, f64) {
    let displacement = target - point;
    let distance = displacement.length().max(1e-9);
    let direction = displacement / distance;
    (direction, distance, light_normal.dot(direction).abs())
}

/// Colour as light: the square of the displayed colour (gamma 2).
pub(super) fn linear(color: Vec3) -> Vec3 {
    color.hadamard(color)
}
