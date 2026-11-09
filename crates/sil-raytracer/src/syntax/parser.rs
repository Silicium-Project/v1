//! Tree building: tokens to entries, with the line of every error.

use super::tokens::{Token, scalar};
use super::{Entry, SyntaxError, Value};

pub(super) struct Parser {
    tokens: Vec<(Token, usize)>,
    position: usize,
}

impl Parser {
    pub(super) fn new(tokens: Vec<(Token, usize)>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens
            .get(self.position + offset)
            .map(|(token, _)| token)
    }

    fn line(&self) -> usize {
        self.tokens
            .get(self.position)
            .or(self.tokens.last())
            .map(|(_, line)| *line)
            .unwrap_or(1)
    }

    /// Entries until the matching `}` (consumed) or the end of the file.
    pub(super) fn block(&mut self, opened_at: Option<usize>) -> Result<Vec<Entry>, SyntaxError> {
        let mut entries = Vec::new();
        loop {
            let line = self.line();
            let Some(token) = self.peek(0).cloned() else {
                return match opened_at {
                    Some(line) => Err(SyntaxError {
                        line,
                        code: "unbalanced_braces",
                        message: "this block is never closed: a `}` is missing".into(),
                    }),
                    None => Ok(entries),
                };
            };
            match token {
                Token::Close => {
                    self.position += 1;
                    return match opened_at {
                        Some(_) => Ok(entries),
                        None => Err(SyntaxError {
                            line,
                            code: "unbalanced_braces",
                            message: "this `}` closes nothing".into(),
                        }),
                    };
                }
                Token::Assign => {
                    // A stray `=` or `:` (e.g. `camera : { ... };` leftovers).
                    self.position += 1;
                }
                Token::Open => {
                    self.position += 1;
                    let inner = self.block(Some(line))?;
                    entries.push(Entry {
                        key: None,
                        value: Value::Block(inner),
                        line,
                    });
                }
                Token::Text(text, quoted) => {
                    self.position += 1;
                    if self.peek(0) == Some(&Token::Assign) {
                        self.position += 1;
                        let value = match self.peek(0).cloned() {
                            Some(Token::Open) => {
                                let opened = self.line();
                                self.position += 1;
                                Value::Block(self.block(Some(opened))?)
                            }
                            Some(Token::Text(value, quoted)) => {
                                self.position += 1;
                                scalar(value, quoted)
                            }
                            _ => {
                                return Err(SyntaxError {
                                    line,
                                    code: "syntax_error",
                                    message: format!("`{text}` has no value after `=`"),
                                });
                            }
                        };
                        entries.push(Entry {
                            key: Some(text),
                            value,
                            line,
                        });
                    } else {
                        entries.push(Entry {
                            key: None,
                            value: scalar(text, quoted),
                            line,
                        });
                    }
                }
            }
        }
    }
}
