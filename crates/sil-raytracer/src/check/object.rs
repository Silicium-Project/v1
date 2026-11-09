//! Checks of the objects themselves: spheres (for the camera), planes (normal) and meshes.

use super::report::Severity;
use super::walk::Walk;
use crate::math::Vec3;
use crate::syntax::{self, BlockExt, Entry, Value};

impl Walk {
    pub(super) fn object(&mut self, shape: &str, entries: &[Entry], line: usize) {
        let n = |key: &str, default: f64| entries.number(key, default, 0.0);
        let offset = entries.block("translate").map_or(Vec3::default(), |t| {
            Vec3::new(
                t.number("x", 0.0, 0.0),
                t.number("y", 0.0, 0.0),
                t.number("z", 0.0, 0.0),
            )
        });
        let scale = match entries.entry("scale").map(|e| &e.value) {
            Some(Value::Number(s)) => s.abs(),
            Some(Value::Block(b)) if syntax::keyframes(b).is_none() => ["x", "y", "z"]
                .iter()
                .map(|k| b.number(k, 1.0, 0.0).abs())
                .fold(0.0, f64::max),
            _ => 1.0,
        };
        if shape == "sphere" {
            let centre = Vec3::new(n("x", 0.0), n("y", 0.0), n("z", 0.0)) + offset;
            self.spheres.push((centre, n("radius", 1.0) * scale, line));
        }
        if shape == "plane" {
            let normal = match entries.block("normal") {
                Some(b) => Vec3::new(
                    b.number("x", 0.0, 0.0),
                    b.number("y", 0.0, 0.0),
                    b.number("z", 1.0, 0.0),
                ),
                None => Vec3::new(n("normal_x", 0.0), n("normal_y", 0.0), n("normal_z", 1.0)),
            };
            if normal.length() <= crate::EPSILON {
                self.push(
                    Severity::Error,
                    "zero_normal",
                    line,
                    "",
                    "this plane's normal is (0, 0, 0): it cannot be drawn".into(),
                    None,
                );
            }
        }
        if shape == "mesh" {
            self.mesh(entries, line);
        }
    }

    fn mesh(&mut self, entries: &[Entry], line: usize) {
        let vertices = entries
            .block("vertices")
            .map(|v| v.numbers())
            .unwrap_or_default();
        let faces = entries
            .block("faces")
            .map(|f| f.numbers())
            .unwrap_or_default();
        let count = vertices.len() / 3;
        let problem = if !vertices.len().is_multiple_of(3) {
            Some("the number of vertex coordinates is not a multiple of 3".to_string())
        } else if !faces.len().is_multiple_of(3) || faces.is_empty() {
            Some("faces must be groups of 3 vertex numbers".to_string())
        } else {
            faces
                .iter()
                .find(|index| **index < 0.0 || index.fract() != 0.0 || **index as usize >= count)
                .map(|index| {
                    format!("a face uses vertex {index}, the mesh has {count} (numbered from 0)")
                })
        };
        if let Some(message) = problem {
            self.push(Severity::Error, "bad_mesh", line, "mesh", message, None);
        }
    }
}
