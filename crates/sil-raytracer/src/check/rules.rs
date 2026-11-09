//! The format as the checker knows it: which keys each block takes, which words and ranges are
//! allowed. Anything not listed here is reported, with the closest known key as suggestion.

use crate::ENGINE_VERSION;

/// Words other raytracers accept and this one does not.
pub(super) const UNSUPPORTED: &[&str] = &[
    "ambient",
    "antialiasing",
    "cone",
    "cube",
    "cylinder",
    "diffuse_light",
    "directional",
    "obj",
    "refraction",
    "shear",
    "spot",
    "texture",
    "torus",
    "transparency",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Root,
    Render,
    Camera,
    Resolution,
    Vector,
    Color,
    Materials,
    Material,
    Primitives,
    Object(&'static str),
    Lights,
    Light(&'static str),
    Animation,
    List,
}

const MATERIAL_VALUES: &[&str] = &[
    "r",
    "g",
    "b",
    "reflection",
    "reflectivity",
    "specular",
    "shininess",
    "roughness",
    "ior",
    "strength",
];

/// Numbers a block accepts.
pub(super) fn values(kind: Kind) -> Vec<&'static str> {
    let mut list: Vec<&'static str> = match kind {
        Kind::Root => vec!["frame"],
        Kind::Render => vec![
            "engine", "samples", "bounces", "exposure", "sky", "seed", "frame", "clamp",
        ],
        Kind::Camera => vec!["fieldOfView", "aperture", "focus_distance", "x", "y", "z"],
        Kind::Resolution => vec!["width", "height"],
        Kind::Vector => vec!["x", "y", "z"],
        Kind::Color => vec!["r", "g", "b"],
        Kind::Material => MATERIAL_VALUES.to_vec(),
        Kind::Object(shape) => {
            let mut own = match shape {
                "sphere" => vec!["x", "y", "z", "radius"],
                "plane" => vec!["x", "y", "z", "normal_x", "normal_y", "normal_z"],
                _ => vec![],
            };
            own.extend_from_slice(MATERIAL_VALUES);
            own
        }
        Kind::Light(shape) => {
            let mut own = vec!["x", "y", "z", "intensity", "r", "g", "b"];
            if shape == "area" {
                own.push("radius");
            }
            own
        }
        Kind::Animation => vec!["frames", "fps"],
        _ => vec![],
    };
    if matches!(kind, Kind::Object(_)) {
        list.push("scale");
    }
    list
}

/// Allowed words: (key, allowed values; empty = any word).
pub(super) fn words(kind: Kind) -> &'static [(&'static str, &'static [&'static str])] {
    const TYPES: &[&str] = &["phong", "diffuse", "metal", "glass", "emissive"];
    match kind {
        Kind::Render => &[
            ("mode", &["whitted", "path"]),
            ("tonemap", &["clamp", "reinhard", "aces"]),
        ],
        Kind::Material => &[("name", &[]), ("type", TYPES)],
        Kind::Object(_) => &[("material", &[]), ("type", TYPES)],
        _ => &[],
    }
}

/// Blocks a block accepts, and the kind of each.
pub(super) fn blocks(kind: Kind) -> Vec<(&'static str, Kind)> {
    let transforms = [
        ("translate", Kind::Vector),
        ("rotate", Kind::Vector),
        ("scale", Kind::Vector),
        ("color", Kind::Color),
    ];
    let mut list = match kind {
        Kind::Root => vec![
            ("render", Kind::Render),
            ("camera", Kind::Camera),
            ("materials", Kind::Materials),
            ("primitives", Kind::Primitives),
            ("lights", Kind::Lights),
            ("animation", Kind::Animation),
            // Empty in older example scenes; nothing is read from it.
            ("effects", Kind::List),
        ],
        Kind::Camera => vec![
            ("resolution", Kind::Resolution),
            ("position", Kind::Vector),
            ("look_at", Kind::Vector),
            ("direction", Kind::Vector),
            ("rotation", Kind::Vector),
            ("up", Kind::Vector),
        ],
        Kind::Materials => vec![("material", Kind::Material)],
        Kind::Material => vec![("color", Kind::Color)],
        Kind::Primitives => ["sphere", "plane", "box", "triangle", "quad", "mesh"]
            .map(|s| (s, Kind::Object(s)))
            .to_vec(),
        Kind::Object(shape) => {
            let mut own = match shape {
                "plane" => vec![("normal", Kind::Vector)],
                "box" => vec![("min", Kind::Vector), ("max", Kind::Vector)],
                "triangle" => vec![
                    ("a", Kind::Vector),
                    ("b", Kind::Vector),
                    ("c", Kind::Vector),
                ],
                "quad" => vec![
                    ("corner", Kind::Vector),
                    ("u", Kind::Vector),
                    ("v", Kind::Vector),
                ],
                "mesh" => vec![("vertices", Kind::List), ("faces", Kind::List)],
                _ => vec![],
            };
            own.extend_from_slice(&transforms);
            own
        }
        Kind::Lights => vec![
            ("point", Kind::Light("point")),
            ("area", Kind::Light("area")),
        ],
        Kind::Light(_) => vec![("color", Kind::Color)],
        _ => vec![],
    };
    list.dedup();
    list
}

/// Range a number must stay in (a warning outside it).
pub(super) fn range(kind: Kind, key: &str) -> Option<(f64, f64)> {
    Some(match (kind, key) {
        (Kind::Render, "samples") => (1.0, 65_536.0),
        (Kind::Render, "bounces") => (0.0, 64.0),
        (Kind::Render, "exposure" | "sky" | "seed" | "clamp") => (0.0, f64::INFINITY),
        (Kind::Render, "engine") => (1.0, f64::from(ENGINE_VERSION)),
        (Kind::Camera, "fieldOfView") => (1.0, 179.0),
        (Kind::Camera, "aperture") => (0.0, f64::INFINITY),
        (Kind::Resolution, _) => (1.0, f64::INFINITY),
        (Kind::Color, _) | (_, "r" | "g" | "b") => (0.0, 255.0),
        (_, "reflection" | "reflectivity") => (0.0, 0.95),
        (_, "specular") => (0.0, 2.0),
        (_, "shininess") => (1.0, 512.0),
        (_, "roughness") => (0.0, 1.0),
        (_, "ior") => (1.0, 3.0),
        (_, "strength" | "intensity") => (0.0, f64::INFINITY),
        (_, "radius") => (1e-4, f64::INFINITY),
        _ => return None,
    })
}

/// The block a key belongs in, when it was written in the wrong place.
pub(super) fn misplaced(key: &str) -> Option<(&str, &str)> {
    match key {
        "sphere" | "plane" | "box" | "triangle" | "quad" | "mesh" => Some((key, "primitives")),
        "point" | "area" => Some((key, "lights")),
        "material" => Some((key, "materials")),
        _ => None,
    }
}
