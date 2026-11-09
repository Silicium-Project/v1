//! Syntax of scenes: `key = value` / `name = { ... }` (`:` works like `=`),
//! read into a tree.
//!
//! It understands comments (`#`, `//`), quoted text, exponents (`1e-3`), lists of
//! bare values (`vertices = { 0 0 0  1 0 0 }`), keyframes (`x = { 0 = 1  48 = 5 }`)
//! and libconfig lists (`( ... )`).

mod block;
mod keyframes;
mod parser;
#[cfg(test)]
mod tests;
mod tokens;

pub use block::BlockExt;
pub use keyframes::{evaluate, keyframes};

use std::fmt;

/// A value: a number, a word (`glass`, `"car body"`) or a block.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Word(String),
    Block(Vec<Entry>),
}

/// One item of a block: `key = value`, or a bare value (`key` is `None`).
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub key: Option<String>,
    pub value: Value,
    /// 1-based line of the key (or of the bare value).
    pub line: usize,
}

/// Syntax error with its line.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntaxError {
    pub line: usize,
    /// `unbalanced_braces` or `syntax_error`.
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// Parses a whole scene file into its top-level entries.
pub fn parse(text: &str) -> Result<Vec<Entry>, SyntaxError> {
    let tokens = tokens::tokenize(text)?;
    parser::Parser::new(tokens).block(None)
}

/// The blocks described by a value: the block itself, or each block of a
/// list (`point = ( { ... }, { ... } )`, the libconfig form).
pub fn items(value: &Value) -> Vec<&[Entry]> {
    match value {
        Value::Block(entries)
            if !entries.is_empty()
                && entries
                    .iter()
                    .all(|e| e.key.is_none() && matches!(e.value, Value::Block(_))) =>
        {
            entries
                .iter()
                .filter_map(|e| match &e.value {
                    Value::Block(inner) => Some(inner.as_slice()),
                    _ => None,
                })
                .collect()
        }
        Value::Block(entries) => vec![entries.as_slice()],
        _ => Vec::new(),
    }
}
