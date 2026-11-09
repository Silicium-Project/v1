//! Whitted lighting: recursive rays for mirrors, glass and metal, plus Phong lighting.

use super::light::{background, fresnel, glossy, phong_light, refract};
use super::{material_of, nearest};
use crate::EPSILON;
use crate::math::Vec3;
use crate::random::Rng;
use crate::scene::{MaterialKind, Scene};

pub(super) fn whitted(
    scene: &Scene,
    origin: Vec3,
    direction: Vec3,
    depth: u32,
    rng: &mut Rng,
) -> Vec3 {
    let Some(hit) = nearest(scene, origin, direction, f64::INFINITY) else {
        return background(scene, direction);
    };
    let material = *material_of(scene, &hit);
    let view = -direction;
    let outside = hit.point + hit.normal * (EPSILON * 4.0);
    let inside = hit.point - hit.normal * (EPSILON * 4.0);
    let deeper = depth < scene.settings.bounces;
    match material.kind {
        MaterialKind::Emissive => material.emission.clamp(),
        MaterialKind::Glass => {
            let eta = if hit.front_face {
                1.0 / material.ior
            } else {
                material.ior
            };
            let cos_i = view.dot(hit.normal).clamp(0.0, 1.0);
            let refracted = refract(direction, hit.normal, eta);
            let reflectance = if refracted.is_some() {
                fresnel(cos_i, eta)
            } else {
                1.0
            };
            let (_, highlight) = phong_light(scene, &hit, view, 256.0, 1.0, rng);
            if !deeper {
                return (background(scene, direction).hadamard(material.color) + highlight).clamp();
            }
            let reflected = whitted(
                scene,
                outside,
                direction.reflect(hit.normal).normalized(),
                depth + 1,
                rng,
            );
            let transmitted = match refracted {
                Some(through) => whitted(scene, inside, through, depth + 1, rng),
                None => Vec3::default(),
            };
            ((reflected * reflectance + transmitted.hadamard(material.color) * (1.0 - reflectance))
                + highlight)
                .clamp()
        }
        MaterialKind::Metal => {
            let shininess: f64 = 2.0 / (material.roughness * material.roughness).max(1e-4);
            let (_, highlight) = phong_light(scene, &hit, view, shininess.min(512.0), 1.0, rng);
            let mut bounce = glossy(
                direction.reflect(hit.normal).normalized(),
                material.roughness,
                rng,
            );
            if bounce.dot(hit.normal) <= 0.0 {
                bounce = direction.reflect(hit.normal).normalized();
            }
            let reflected = if deeper {
                whitted(scene, outside, bounce, depth + 1, rng)
            } else {
                background(scene, bounce)
            };
            (reflected.hadamard(material.color) + highlight.hadamard(material.color)).clamp()
        }
        MaterialKind::Phong | MaterialKind::Diffuse => {
            let (diffuse, highlight) = phong_light(
                scene,
                &hit,
                view,
                material.shininess,
                material.specular,
                rng,
            );
            let local = material.color.hadamard(diffuse) + highlight;
            if !deeper || material.reflectivity <= 0.0 {
                return local.clamp();
            }
            let bounce = glossy(
                direction.reflect(hit.normal).normalized(),
                material.roughness,
                rng,
            );
            let reflected = whitted(scene, outside, bounce, depth + 1, rng);
            (local * (1.0 - material.reflectivity) + reflected * material.reflectivity).clamp()
        }
    }
}
