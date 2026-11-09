//! Keyframes: a value that changes with the animation frame.

use super::{Entry, Value};

/// Keyframes of a value: `{ 0 = 1  48 = 5  ease = smooth }`, sorted by frame.
pub fn keyframes(entries: &[Entry]) -> Option<(Vec<(f64, f64)>, bool)> {
    let mut keys = Vec::new();
    let mut smooth = false;
    for entry in entries {
        let key = entry.key.as_deref()?;
        if key == "ease" {
            smooth = matches!(&entry.value, Value::Word(word) if word == "smooth");
            continue;
        }
        let frame = key.parse::<f64>().ok()?;
        let Value::Number(value) = entry.value else {
            return None;
        };
        keys.push((frame, value));
    }
    if keys.is_empty() {
        return None;
    }
    keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    Some((keys, smooth))
}

/// Value of a number or of keyframes at `frame` (clamped to the first and
/// last keys, linear or smoothstep in between).
pub fn evaluate(value: &Value, frame: f64) -> Option<f64> {
    match value {
        Value::Number(number) => Some(*number),
        Value::Block(entries) => {
            let (keys, smooth) = keyframes(entries)?;
            let first = keys[0];
            let last = keys[keys.len() - 1];
            if frame <= first.0 {
                return Some(first.1);
            }
            if frame >= last.0 {
                return Some(last.1);
            }
            let index = keys.iter().position(|(at, _)| *at > frame)?;
            let (a, b) = (keys[index - 1], keys[index]);
            let mut t = (frame - a.0) / (b.0 - a.0);
            if smooth {
                t = t * t * (3.0 - 2.0 * t);
            }
            Some(a.1 + (b.1 - a.1) * t)
        }
        Value::Word(_) => None,
    }
}
