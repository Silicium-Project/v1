//! The walk over the syntax tree: each block against its allowed keys, values and ranges.

use super::report::{Issue, Severity, issue, suggest};
use super::rules::{Kind, UNSUPPORTED, blocks, misplaced, range, values, words};
use crate::math::Vec3;
use crate::syntax::{self, BlockExt, Entry, Value};
use std::collections::HashSet;

/// What the walk collects: the issues, and what the whole-scene rules need afterwards.
pub(super) struct Walk {
    pub(super) issues: Vec<Issue>,
    pub(super) material_names: HashSet<String>,
    /// Material names used by objects, with the line of each use.
    pub(super) material_uses: Vec<(String, usize)>,
    pub(super) lights: usize,
    pub(super) resolution_line: Option<usize>,
    pub(super) camera_line: Option<usize>,
    /// Centre, radius and line of each sphere (transforms other than
    /// `translate` and `scale` ignored), to warn about spheres behind the camera.
    pub(super) spheres: Vec<(Vec3, f64, usize)>,
}

impl Walk {
    pub(super) fn new() -> Self {
        Walk {
            issues: Vec::new(),
            material_names: HashSet::new(),
            material_uses: Vec::new(),
            lights: 0,
            resolution_line: None,
            camera_line: None,
            spheres: Vec::new(),
        }
    }

    pub(super) fn push(
        &mut self,
        severity: Severity,
        code: &'static str,
        line: usize,
        key: &str,
        message: String,
        suggestion: Option<String>,
    ) {
        let mut found = issue(severity, code, line, key, &message);
        found.suggestion = suggestion;
        self.issues.push(found);
    }

    fn number(&mut self, kind: Kind, key: &str, entry: &Entry) {
        let ok = match &entry.value {
            Value::Number(value) => {
                if let Some((min, max)) = range(kind, key)
                    && !(min..=max).contains(value)
                {
                    let bounds = if max.is_infinite() {
                        format!("at least {min}")
                    } else {
                        format!("between {min} and {max}")
                    };
                    self.push(
                        Severity::Warning,
                        "out_of_range",
                        entry.line,
                        key,
                        format!("`{key}` should be {bounds}, got {value}"),
                        None,
                    );
                }
                true
            }
            Value::Block(inner) => syntax::keyframes(inner).is_some(),
            Value::Word(_) => false,
        };
        if !ok {
            self.push(
                Severity::Error,
                "not_a_number",
                entry.line,
                key,
                format!("`{key}` must be a number or keyframes `{{ 0 = 1  24 = 2 }}`"),
                None,
            );
        }
    }

    pub(super) fn block(&mut self, kind: Kind, entries: &[Entry]) {
        let numbers = values(kind);
        let allowed_words = words(kind);
        let children = blocks(kind);
        let mut seen = HashSet::new();
        if matches!(kind, Kind::Vector | Kind::Color) && entries.iter().all(|e| e.key.is_none()) {
            if entries.len() != 3 || entries.iter().any(|e| !matches!(e.value, Value::Number(_))) {
                let line = entries.first().map_or(0, |e| e.line);
                self.push(
                    Severity::Error,
                    "not_a_number",
                    line,
                    "",
                    "write three numbers or `x = … y = … z = …`".into(),
                    None,
                );
            }
            return;
        }
        if kind == Kind::List {
            return;
        }
        for entry in entries {
            let Some(key) = entry.key.as_deref() else {
                if kind != Kind::Root || !matches!(entry.value, Value::Block(_)) {
                    self.push(
                        Severity::Warning,
                        "unexpected_token",
                        entry.line,
                        "",
                        "a value without a key is ignored".into(),
                        None,
                    );
                }
                continue;
            };
            let block_kind = children
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, k)| *k);
            let repeatable = matches!(
                block_kind,
                Some(Kind::Object(_) | Kind::Light(_) | Kind::Material)
            );
            if !repeatable && !seen.insert(key.to_string()) {
                self.push(
                    Severity::Warning,
                    "duplicate_key",
                    entry.line,
                    key,
                    format!("`{key}` is given twice: only the first one is used"),
                    None,
                );
                continue;
            }
            match (&entry.value, block_kind) {
                (Value::Block(inner), Some(child))
                    if !(key == "scale" && syntax::keyframes(inner).is_some()) =>
                {
                    match child {
                        Kind::Resolution => {
                            self.resolution_line = self.resolution_line.or(Some(entry.line))
                        }
                        Kind::Camera => self.camera_line = self.camera_line.or(Some(entry.line)),
                        _ => {}
                    }
                    // `point = ( { ... }, { ... } )`: one item per block of the list.
                    let parts = if repeatable {
                        syntax::items(&entry.value)
                    } else {
                        vec![inner.as_slice()]
                    };
                    for part in parts {
                        if matches!(child, Kind::Light(_)) {
                            self.lights += 1;
                        }
                        self.block(child, part);
                        if let Kind::Object(shape) = child {
                            self.object(shape, part, entry.line);
                        }
                        if matches!(child, Kind::Object(_) | Kind::Material) {
                            self.reflection_twice(part, entry.line);
                        }
                    }
                }
                _ if numbers.contains(&key) => self.number(kind, key, entry),
                (value, _) if allowed_words.iter().any(|(name, _)| *name == key) => {
                    let allowed = allowed_words
                        .iter()
                        .find(|(name, _)| *name == key)
                        .map(|(_, a)| *a)
                        .unwrap_or(&[]);
                    match value {
                        Value::Word(word) => {
                            if !allowed.is_empty() && !allowed.contains(&word.as_str()) {
                                self.push(
                                    Severity::Error,
                                    "invalid_value",
                                    entry.line,
                                    key,
                                    format!("`{key}` can be {}, not `{word}`", allowed.join(", ")),
                                    suggest(word, allowed),
                                );
                            }
                            if key == "name" && kind == Kind::Material {
                                self.material_names.insert(word.clone());
                            }
                            if key == "material" && matches!(kind, Kind::Object(_)) {
                                self.material_uses.push((word.clone(), entry.line));
                            }
                        }
                        _ => self.push(
                            Severity::Error,
                            "invalid_value",
                            entry.line,
                            key,
                            format!("`{key}` must be a word"),
                            None,
                        ),
                    }
                }
                (Value::Block(_), None) | (_, Some(_)) => {
                    let candidates: Vec<&str> = children.iter().map(|(name, _)| *name).collect();
                    if UNSUPPORTED.contains(&key) {
                        self.push(
                            Severity::Warning,
                            "unsupported",
                            entry.line,
                            key,
                            format!("`{key}` is not supported by this renderer"),
                            None,
                        );
                    } else if let Some((_, place)) = misplaced(key) {
                        self.push(
                            Severity::Warning,
                            "misplaced",
                            entry.line,
                            key,
                            format!("`{key}` belongs in `{place}`"),
                            None,
                        );
                    } else {
                        self.push(
                            Severity::Warning,
                            "unknown_block",
                            entry.line,
                            key,
                            format!("unknown block `{key}`: it is ignored"),
                            suggest(key, &candidates),
                        );
                    }
                }
                _ => {
                    let mut candidates: Vec<&str> = numbers.clone();
                    candidates.extend(allowed_words.iter().map(|(name, _)| *name));
                    candidates.extend(children.iter().map(|(name, _)| *name));
                    if UNSUPPORTED.contains(&key) {
                        self.push(
                            Severity::Warning,
                            "unsupported",
                            entry.line,
                            key,
                            format!("`{key}` is not supported by this renderer"),
                            None,
                        );
                    } else {
                        self.push(
                            Severity::Warning,
                            "unknown_key",
                            entry.line,
                            key,
                            format!("unknown key `{key}`: it is ignored"),
                            suggest(key, &candidates),
                        );
                    }
                }
            }
        }
    }

    fn reflection_twice(&mut self, entries: &[Entry], line: usize) {
        if entries.entry("reflection").is_some() && entries.entry("reflectivity").is_some() {
            self.push(
                Severity::Warning,
                "duplicate_reflection",
                line,
                "reflectivity",
                "`reflection` and `reflectivity` are the same setting: `reflectivity` wins".into(),
                None,
            );
        }
    }
}
