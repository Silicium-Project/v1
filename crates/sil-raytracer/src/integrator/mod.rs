//! Colour of a pixel: several camera rays per pixel, each traced with the Whitted model (Phong
//! highlights, mirror reflections, shadows, plus glass, metal and area lights) or with Monte-Carlo
//! path tracing, then averaged, tone-mapped and gamma-corrected (square root).

mod light;
mod path;
mod whitted;

use crate::math::Vec3;
use crate::random::Rng;
use crate::scene::{Material, Mode, Scene, Tonemap};
use path::path;
use whitted::whitted;

/// Hit seen from a ray.
#[derive(Clone, Copy, Debug)]
pub(super) struct Hit {
    pub(super) distance: f64,
    pub(super) point: Vec3,
    /// Facing the incoming ray.
    pub(super) normal: Vec3,
    /// True when the ray comes from the outside of the surface.
    pub(super) front_face: bool,
    pub(super) shape: usize,
}

/// The nearest surface along a ray, within `limit`.
pub(super) fn nearest(scene: &Scene, origin: Vec3, direction: Vec3, limit: f64) -> Option<Hit> {
    let mut best = scene.bvh.intersect(&scene.shapes, origin, direction, limit);
    for &plane in &scene.planes {
        if let Some((distance, normal)) = scene.shapes[plane].intersect(origin, direction)
            && distance < best.map_or(limit, |hit| hit.0)
        {
            best = Some((distance, normal, plane));
        }
    }
    best.map(|(distance, outward, shape)| {
        let front_face = outward.dot(direction) < 0.0;
        Hit {
            distance,
            point: origin + direction * distance,
            normal: if front_face { outward } else { -outward },
            front_face,
            shape,
        }
    })
}

pub(super) fn material_of<'a>(scene: &'a Scene, hit: &Hit) -> &'a Material {
    &scene.materials[scene.shape_material[hit.shape]]
}

fn tonemap(value: f64, tonemap: Tonemap) -> f64 {
    match tonemap {
        Tonemap::Clamp => value,
        Tonemap::Reinhard => value / (1.0 + value),
        // Narkowicz's fit of the ACES filmic curve.
        Tonemap::Aces => (value * (2.51 * value + 0.03)) / (value * (2.43 * value + 0.59) + 0.14),
    }
    .clamp(0.0, 1.0)
}

/// RGB bytes of pixel (`x`, `y`) of the full image.
pub fn pixel(scene: &Scene, x: usize, y: usize) -> [u8; 3] {
    let settings = &scene.settings;
    let samples = settings.samples.max(1);
    let grid = (f64::from(samples)).sqrt().floor() as u32;
    let strata = grid * grid;
    let frame = settings.frame.round() as i64;
    let camera = &scene.camera;
    let aspect = scene.width as f64 / scene.height as f64;
    let mut sum = Vec3::default();
    for sample in 0..samples {
        let mut rng = Rng::for_sample(x, y, sample, frame, settings.seed);
        let (dx, dy) = if samples == 1 {
            (0.5, 0.5)
        } else if sample < strata {
            (
                (f64::from(sample % grid) + rng.uniform()) / f64::from(grid),
                (f64::from(sample / grid) + rng.uniform()) / f64::from(grid),
            )
        } else {
            (rng.uniform(), rng.uniform())
        };
        let px = (2.0 * (x as f64 + dx) / scene.width as f64 - 1.0) * aspect * camera.tan_half_fov;
        let py = (1.0 - 2.0 * (y as f64 + dy) / scene.height as f64) * camera.tan_half_fov;
        let mut direction = (camera.forward + camera.right * px + camera.up * py).normalized();
        let mut origin = camera.origin;
        if camera.aperture > 0.0 {
            let focus =
                origin + direction * (camera.focus_distance / direction.dot(camera.forward));
            let (u, v) = rng.in_disk();
            origin =
                origin + camera.right * (u * camera.aperture) + camera.up * (v * camera.aperture);
            direction = (focus - origin).normalized();
        }
        let color = match settings.mode {
            Mode::Whitted => whitted(scene, origin, direction, 0, &mut rng),
            Mode::Path => path(scene, origin, direction, &mut rng),
        };
        if color.x.is_finite() && color.y.is_finite() && color.z.is_finite() {
            sum = sum + color;
        }
    }
    let mean = sum * (settings.exposure / f64::from(samples));
    [mean.x, mean.y, mean.z].map(|channel| {
        (tonemap(channel.max(0.0), settings.tonemap).sqrt() * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}
