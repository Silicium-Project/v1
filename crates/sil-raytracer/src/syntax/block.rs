//! Helpers to read a block of entries.

use super::keyframes::evaluate;
use super::{Entry, Value};

pub trait BlockExt {
    fn entries(&self) -> &[Entry];

    /// First entry named `key`.
    fn entry(&self, key: &str) -> Option<&Entry> {
        self.entries()
            .iter()
            .find(|entry| entry.key.as_deref() == Some(key))
    }

    /// Every entry named `key`, in order.
    fn all<'a>(&'a self, key: &'a str) -> Box<dyn Iterator<Item = &'a Entry> + 'a> {
        Box::new(
            self.entries()
                .iter()
                .filter(move |entry| entry.key.as_deref() == Some(key)),
        )
    }

    /// The block named `key`, if any.
    fn block(&self, key: &str) -> Option<&[Entry]> {
        match &self.entry(key)?.value {
            Value::Block(entries) => Some(entries),
            _ => None,
        }
    }

    /// The word named `key`, if any.
    fn word(&self, key: &str) -> Option<&str> {
        match &self.entry(key)?.value {
            Value::Word(word) => Some(word),
            _ => None,
        }
    }

    /// Number `key` at animation frame `frame` (keyframes are interpolated).
    fn number(&self, key: &str, default: f64, frame: f64) -> f64 {
        self.entry(key)
            .and_then(|entry| evaluate(&entry.value, frame))
            .unwrap_or(default)
    }

    /// Bare numbers of the block (`vertices = { 0 0 0  1 0 0 }`).
    fn numbers(&self) -> Vec<f64> {
        self.entries()
            .iter()
            .filter(|entry| entry.key.is_none())
            .filter_map(|entry| match entry.value {
                Value::Number(number) => Some(number),
                _ => None,
            })
            .collect()
    }
}

impl BlockExt for [Entry] {
    fn entries(&self) -> &[Entry] {
        self
    }
}

impl BlockExt for Vec<Entry> {
    fn entries(&self) -> &[Entry] {
        self
    }
}
