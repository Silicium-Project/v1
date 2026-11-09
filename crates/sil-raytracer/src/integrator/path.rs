//! Monte-Carlo path tracing: one random path per camera ray, with the light it finds.

use super::light::{background, direct_light, fresnel, glossy, linear, refract};
use super::{material_of, nearest};
use crate::EPSILON;
use crate::math::Vec3;
use crate::random::Rng;
use crate::scene::{MaterialKind, Scene};
use std::f64::consts::PI;

pub(super) fn path(scene: &Scene, mut origin: Vec3, mut direction: Vec3, rng: &mut Rng) -> Vec3 {
    let mut throughput = Vec3::new(1.0, 1.0, 1.0);
    let mut radiance = Vec3::default();
    let mut count_emission = true;
    // Once a path has bounced off a rough surface (diffuse, glossy coat,
    // rough metal), a light reached afterwards through glass or a mirror is
    // a rare, very bright sample (a "firefly"): its contribution is capped
    // at `render.clamp` (biased, but clean images).
    let mut after_rough = false;
    let cap = scene.settings.clamp;
    for depth in 0..=scene.settings.bounces {
        let Some(hit) = nearest(scene, origin, direction, f64::INFINITY) else {
            radiance = radiance + throughput.hadamard(background(scene, direction));
            break;
        };
        let material = *material_of(scene, &hit);
        if material.kind == MaterialKind::Emissive {
            if count_emission {
                let mut added = throughput.hadamard(material.emission);
                if after_rough {
                    added = Vec3::new(added.x.min(cap), added.y.min(cap), added.z.min(cap));
                }
                radiance = radiance + added;
            }
            break;
        }
        if depth == scene.settings.bounces {
            break;
        }
        let outside = hit.point + hit.normal * (EPSILON * 4.0);
        let reflected = direction.reflect(hit.normal).normalized();
        let next = match material.kind {
            MaterialKind::Phong | MaterialKind::Diffuse => {
                let coat = if material.kind == MaterialKind::Phong {
                    (material.reflectivity + 0.04 * material.specular).clamp(0.0, 0.95)
                } else {
                    0.0
                };
                if rng.uniform() < coat {
                    let roughness =
                        (2.0_f64 / (material.shininess + 2.0)).sqrt() * 0.5 + material.roughness;
                    let bounce = glossy(reflected, roughness.min(1.0), rng);
                    if bounce.dot(hit.normal) <= 0.0 {
                        break;
                    }
                    count_emission = true;
                    after_rough = true;
                    (outside, bounce)
                } else {
                    let albedo = linear(material.color);
                    let light = direct_light(scene, hit.point, hit.normal, rng);
                    radiance = radiance + throughput.hadamard(albedo).hadamard(light) / PI;
                    throughput = throughput.hadamard(albedo);
                    count_emission = false;
                    after_rough = true;
                    (outside, rng.cosine_hemisphere(hit.normal))
                }
            }
            MaterialKind::Metal => {
                let bounce = glossy(reflected, material.roughness, rng);
                if bounce.dot(hit.normal) <= 0.0 {
                    break;
                }
                throughput = throughput.hadamard(linear(material.color));
                count_emission = true;
                after_rough |= material.roughness > 0.05;
                (outside, bounce)
            }
            MaterialKind::Glass => {
                let eta = if hit.front_face {
                    1.0 / material.ior
                } else {
                    material.ior
                };
                let cos_i = (-direction).dot(hit.normal).clamp(0.0, 1.0);
                throughput = throughput.hadamard(material.color);
                count_emission = true;
                match refract(direction, hit.normal, eta) {
                    Some(through) if rng.uniform() >= fresnel(cos_i, eta) => {
                        (hit.point - hit.normal * (EPSILON * 4.0), through)
                    }
                    _ => (outside, reflected),
                }
            }
            MaterialKind::Emissive => break,
        };
        if depth >= 3 {
            let survival = throughput.max_component().clamp(0.05, 1.0);
            if rng.uniform() > survival {
                break;
            }
            throughput = throughput / survival;
        }
        (origin, direction) = next;
    }
    radiance
}
