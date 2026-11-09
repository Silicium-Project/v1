//! The whole check: the walk over the blocks, then the rules that need the built scene.

use super::report::{LARGE_PIXELS, Report, Severity, Summary, issue, report, suggest};
use super::walk::Walk;
use crate::ENGINE_VERSION;
use crate::geometry::Shape;
use crate::scene::{Mode, build, scene_resolution};
use crate::syntax::{self, BlockExt};

/// Checks the text of a scene file.
pub fn check_scene(text: &str) -> Report {
    let mut summary = Summary::defaults();
    let root = match syntax::parse(text) {
        Ok(root) => root,
        Err(error) => {
            let found = issue(Severity::Error, error.code, error.line, "", &error.message);
            return report(vec![found], summary);
        }
    };
    let mut walk = Walk::new();
    walk.block(super::rules::Kind::Root, &root);
    undefined_materials(&mut walk);
    too_new_engine(&mut walk, &root);
    match build(text, None) {
        Ok(scene) => {
            scene_rules(&mut walk, text, &scene);
            fill_summary(&mut summary, &scene, &root);
        }
        Err(error) => {
            if !walk.issues.iter().any(|i| i.severity == Severity::Error) {
                walk.push(Severity::Error, "engine_error", 0, "", error, None);
            }
        }
    }
    report(walk.issues, summary)
}

/// Objects naming a material that is never declared.
fn undefined_materials(walk: &mut Walk) {
    let uses = std::mem::take(&mut walk.material_uses);
    let names: Vec<String> = walk.material_names.iter().cloned().collect();
    let candidates: Vec<&str> = names.iter().map(String::as_str).collect();
    for (name, line) in uses {
        if !walk.material_names.contains(&name) {
            walk.push(
                Severity::Error,
                "unknown_material",
                line,
                &name,
                format!("no material named `{name}` in `materials`"),
                suggest(&name, &candidates),
            );
        }
    }
}

/// A scene asking for a newer engine than this one.
fn too_new_engine(walk: &mut Walk, root: &[syntax::Entry]) {
    let Some(engine_entry) = root.block("render").and_then(|r| r.entry("engine")) else {
        return;
    };
    let Some(engine) = syntax::evaluate(&engine_entry.value, 0.0) else {
        return;
    };
    if engine > f64::from(ENGINE_VERSION) {
        walk.push(
            Severity::Error,
            "engine_too_new",
            engine_entry.line,
            "engine",
            format!("the scene needs engine {engine}, this one is {ENGINE_VERSION}"),
            None,
        );
    }
}

/// The rules that need the built scene: spheres behind the camera, the resolution, emptiness.
fn scene_rules(walk: &mut Walk, text: &str, scene: &crate::scene::Scene) {
    let camera = scene.camera;
    for (centre, radius, line) in std::mem::take(&mut walk.spheres) {
        if (centre - camera.origin).dot(camera.forward) + radius <= 0.0 {
            walk.push(
                Severity::Warning,
                "behind_camera",
                line,
                "x",
                "this sphere is behind the camera: it will not be seen".into(),
                None,
            );
        }
    }
    let planned = scene_resolution(text);
    match walk.resolution_line {
        None => walk.push(
            Severity::Error,
            "missing_resolution",
            walk.camera_line.unwrap_or(0),
            "resolution",
            "no `resolution = { width = W height = H }` in the camera".into(),
            None,
        ),
        Some(line) if planned != (scene.width, scene.height) => walk.push(
            Severity::Error,
            "resolution_mismatch",
            line,
            "resolution",
            format!(
                "the bands would be cut for {}x{} but the image is {}x{}: write `resolution = {{ width = W height = H }}`, width first",
                planned.0, planned.1, scene.width, scene.height
            ),
            None,
        ),
        Some(line) => {
            if scene.width * scene.height > LARGE_PIXELS {
                walk.push(
                    Severity::Warning,
                    "resolution_large",
                    line,
                    "resolution",
                    "more than 8K: the render will be long and costly".into(),
                    None,
                );
            }
        }
    }
    let emitting = scene.emitters.len();
    let lit = !scene.points.is_empty()
        || emitting > 0
        || (scene.settings.mode == Mode::Path && scene.settings.sky > 0.0);
    if scene.shapes.len()
        == scene
            .emitters
            .iter()
            .filter(|e| e.intensity.is_some())
            .count()
    {
        walk.push(
            Severity::Warning,
            "empty_scene",
            0,
            "",
            "no object: the image will only show the sky".into(),
            None,
        );
    }
    if walk.lights == 0 && !lit {
        walk.push(
            Severity::Warning,
            "dark_scene",
            0,
            "",
            "no light, no glowing object and no sky: the image will be black".into(),
            None,
        );
    } else if walk.lights == 0 && scene.settings.mode == Mode::Whitted && emitting == 0 {
        walk.push(
            Severity::Warning,
            "no_light",
            0,
            "",
            "no light: a default white light at (4, 0, 5) is added".into(),
            None,
        );
    }
}

/// What the summary reports, read from the built scene and the root block.
fn fill_summary(summary: &mut Summary, scene: &crate::scene::Scene, root: &[syntax::Entry]) {
    summary.width = scene.width;
    summary.height = scene.height;
    summary.spheres = scene
        .shapes
        .iter()
        .enumerate()
        .filter(|(i, s)| {
            matches!(s, Shape::Sphere { .. })
                && !scene
                    .emitters
                    .iter()
                    .any(|e| e.shape == *i && e.intensity.is_some())
        })
        .count();
    summary.planes = scene.planes.len();
    summary.triangles = scene
        .shapes
        .iter()
        .filter(|s| matches!(s, Shape::Triangle { .. } | Shape::Quad { .. }))
        .count();
    summary.lights = scene.points.len() + scene.emitters.len();
    summary.samples = scene.settings.samples;
    summary.mode = if scene.settings.mode == Mode::Path {
        "path"
    } else {
        "whitted"
    };
    summary.field_of_view = root
        .block("camera")
        .map_or(55.0, |c| c.number("fieldOfView", 55.0, 0.0));
}
