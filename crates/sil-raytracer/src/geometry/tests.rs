use super::*;
use crate::math::Vec3;

fn brute(shapes: &[Shape], origin: Vec3, direction: Vec3) -> Option<(f64, usize)> {
    let mut best: Option<(f64, usize)> = None;
    for (index, shape) in shapes.iter().enumerate() {
        if let Some((distance, _)) = shape.intersect(origin, direction)
            && best.is_none_or(|(current, current_index)| {
                distance < current || (distance == current && index < current_index)
            })
        {
            best = Some((distance, index));
        }
    }
    best
}

#[test]
fn bvh_finds_the_same_hits_as_testing_every_shape() {
    let mut shapes = Vec::new();
    for i in 0..40 {
        let f = i as f64;
        shapes.push(Shape::triangle(
            Vec3::new(5.0 + (f * 0.37).sin(), f * 0.1 - 2.0, (f * 0.7).cos()),
            Vec3::new(5.5 + (f * 0.13).cos(), f * 0.1 - 1.6, (f * 0.3).sin()),
            Vec3::new(4.8, f * 0.1 - 1.8, 0.6 + (f * 0.2).sin()),
        ));
        shapes.push(Shape::Sphere {
            center: Vec3::new(7.0 + f * 0.05, (f * 0.9).sin() * 2.0, (f * 0.4).cos()),
            radius: 0.15,
        });
    }
    let bvh = Bvh::build(&shapes, (0..shapes.len()).collect());
    for y in 0..30 {
        for z in 0..30 {
            let direction =
                Vec3::new(1.0, (y as f64 - 15.0) / 12.0, (z as f64 - 15.0) / 12.0).normalized();
            let fast = bvh
                .intersect(&shapes, Vec3::default(), direction, f64::INFINITY)
                .map(|h| (h.0, h.2));
            assert_eq!(fast, brute(&shapes, Vec3::default(), direction));
        }
    }
}

#[test]
fn transforms_scale_rotate_then_translate() {
    let t = Transform::new(
        Vec3::new(2.0, 2.0, 2.0),
        Vec3::new(0.0, 0.0, 90.0),
        Vec3::new(1.0, 0.0, 0.0),
    );
    let p = t.point(Vec3::new(1.0, 0.0, 0.0));
    assert!((p - Vec3::new(1.0, 2.0, 0.0)).length() < 1e-12);
    let n = t.normal(Vec3::new(1.0, 0.0, 0.0));
    assert!((n - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-12);
}

#[test]
fn quads_and_triangles_are_hit_inside_only() {
    let quad = Shape::quad(
        Vec3::new(5.0, -1.0, -1.0),
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::new(0.0, 0.0, 2.0),
    );
    assert!(
        quad.intersect(Vec3::default(), Vec3::new(1.0, 0.0, 0.0))
            .is_some()
    );
    assert!(
        quad.intersect(Vec3::default(), Vec3::new(1.0, 0.5, 0.0).normalized())
            .is_none()
    );
    assert!((quad.area() - 4.0).abs() < 1e-12);
}
