#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The engine must stay usable by the network: deterministic pixels whatever the band, and pinned
//! outputs so that a change of the pixels (which would make nodes of different versions disagree)
//! is a deliberate one. The pinned hashes come from the V0 engine (version 2).

use sha2::{Digest, Sha256};
use sil_raytracer::{geometry::Shape, render::render_rows, scene::build};

/// Width and height of the image a scene declares.
fn build_scene(text: &str) -> (u32, u32) {
    let scene = build(text, None).expect("valid scene");
    (scene.width as u32, scene.height as u32)
}

/// Height of the first shape at animation frame `frame`.
fn shape_centre_y(text: &str, frame: f64) -> f64 {
    let scene = build(text, Some(frame)).expect("valid scene");
    match scene.shapes[0] {
        Shape::Sphere { center, .. } => center.y,
        _ => unreachable!("the test scene starts with a sphere"),
    }
}

const DEMO: &str = include_str!("../scenes/demo-scene.cfg");

const PATH_SCENE: &str = "render : { mode = path samples = 4 bounces = 4 }
camera : {
  resolution = { width = 32 height = 20 }
  position = { x = 0 y = 0 z = 0.5 }
  look_at = { x = 5 y = 0 z = 0 }
  fieldOfView = 50
}
materials : {
  material = { name = glass type = glass ior = 1.5 }
  material = { name = lamp type = emissive strength = 4 }
}
primitives : {
  plane = { z = -1 type = diffuse }
  sphere = { x = 5 y = 0.8 z = 0 radius = 0.8 material = glass }
  box = { translate = { x = 6 y = -1.2 z = -0.5 } rotate = { z = 25 } type = metal roughness = 0.2 }
  quad = { corner = { x = 4 y = -1 z = 3 } u = { x = 1 y = 0 z = 0 } v = { x = 0 y = 2 z = 0 } material = lamp }
}
lights : { area = { x = 2 y = -3 z = 3 radius = 0.3 intensity = 300 } }
";

const PINNED_DEMO_HASH: &str = "514b05b3b50f59436fb8a8f68c2ea2fbbfc2e6d7369a646ffb54f40180f9db8f";
const PINNED_PATH_HASH: &str = "138b6ed2add7e56c7a46f6b6b80f03810a54c4ec6cafc6f00c399e739a8e1f19";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The whole image of a scene, rendered as one band.
fn render_whole(text: &str) -> Vec<u8> {
    let scene = build_scene(text);
    render_rows(text, scene.0, scene.1, 0, scene.1).expect("valid scene")
}

fn small_demo() -> String {
    DEMO.replace("width = 320 height = 200", "width = 48 height = 30")
}

#[test]
fn scenes_without_render_block_are_pinned() {
    // The classic look (one ray per pixel, Whitted lighting) of the demo scene.
    let pixels = render_whole(&small_demo());
    assert_eq!(sha256_hex(&pixels), PINNED_DEMO_HASH);
}

#[test]
fn engine_2_output_is_pinned() {
    // Changing this hash changes the pixels: nodes of different versions would then reject each
    // other's bands. Do it only with a new `ENGINE_VERSION`.
    let pixels = render_whole(PATH_SCENE);
    assert_eq!(sha256_hex(&pixels), PINNED_PATH_HASH);
}

#[test]
fn a_band_is_the_same_rows_of_the_full_image() {
    let full = render_whole(PATH_SCENE);
    let band = render_rows(PATH_SCENE, 32, 20, 3, 14).expect("valid band");
    assert_eq!(band, full[3 * 32 * 3..14 * 32 * 3]);
    let again = render_whole(PATH_SCENE);
    assert_eq!(full, again, "rendering twice gives the same bytes");
}

#[test]
fn several_samples_smooth_the_edges() {
    let one = render_whole(&format!("render : {{ samples = 1 }}\n{}", small_demo()));
    let many = render_whole(&format!("render : {{ samples = 16 }}\n{}", small_demo()));
    assert_ne!(one, many);
    // Averaged edge pixels take values between the two sides: the jumps between neighbouring
    // pixels get smaller overall.
    let jumps = |pixels: &[u8]| -> u64 {
        pixels
            .chunks(48 * 3)
            .flat_map(|row| row.windows(4).map(|w| u64::from(w[0].abs_diff(w[3]))))
            .sum()
    };
    assert!(jumps(&many) < jumps(&one));
}

#[test]
fn look_at_the_default_direction_changes_nothing() {
    let base = format!("render : {{ }}\n{}", small_demo());
    let aimed = base.replace(
        "fieldOfView = 55.0",
        "fieldOfView = 55.0\n\tlook_at = { x = 9 y = 0 z = 0 }",
    );
    assert_ne!(base, aimed);
    let a = render_whole(&base);
    let b = render_whole(&aimed);
    let worst = a
        .iter()
        .zip(&b)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0);
    assert!(worst <= 1);
}

#[test]
fn keyframes_move_objects_between_frames() {
    let text = "render : { }\ncamera : { resolution = { width = 24 height = 16 } }\n\
                primitives : { sphere = { x = 5 y = { 0 = -2  10 = 2 } radius = 0.8 } }";
    assert_eq!(
        (
            shape_centre_y(text, 0.0),
            shape_centre_y(text, 5.0),
            shape_centre_y(text, 10.0),
            shape_centre_y(text, 20.0)
        ),
        (-2.0, 0.0, 2.0, 2.0)
    );
}

#[test]
fn a_newer_engine_is_refused_instead_of_drawn_differently() {
    let error = render_rows(
        "render : { engine = 3 }\ncamera : { resolution = { width = 4 height = 4 } }",
        4,
        4,
        0,
        4,
    )
    .unwrap_err();
    assert!(error.to_string().contains("scene_requires_engine:3"));
}

#[test]
fn a_band_outside_the_image_or_of_another_size_is_refused() {
    assert!(render_rows(PATH_SCENE, 32, 20, 5, 25).is_err());
    assert!(render_rows(PATH_SCENE, 16, 20, 0, 4).is_err());
}
