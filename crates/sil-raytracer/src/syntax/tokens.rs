//! Lexical analysis: text to tokens, each with its line.

use super::{SyntaxError, Value};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Token {
    Open,
    Close,
    Assign,
    /// Text, and whether it was quoted (a quoted number stays a word).
    Text(String, bool),
}

pub(super) fn tokenize(text: &str) -> Result<Vec<(Token, usize)>, SyntaxError> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    let mut line = 1;
    let mut current = String::new();
    let flush = |current: &mut String, tokens: &mut Vec<(Token, usize)>, line: usize| {
        if !current.is_empty() {
            tokens.push((Token::Text(std::mem::take(current), false), line));
        }
    };
    while let Some(character) = chars.next() {
        let sign_allowed = current.is_empty()
            || (current.ends_with(['e', 'E'])
                && current
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+'));
        match character {
            c if c.is_ascii_alphanumeric() || c == '_' || c == '.' => current.push(c),
            '-' | '+' if sign_allowed => current.push(character),
            '#' => {
                flush(&mut current, &mut tokens, line);
                while chars.peek().is_some_and(|c| *c != '\n') {
                    chars.next();
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                flush(&mut current, &mut tokens, line);
                while chars.peek().is_some_and(|c| *c != '\n') {
                    chars.next();
                }
            }
            '"' => {
                flush(&mut current, &mut tokens, line);
                let start = line;
                let mut word = String::new();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\n') | None => {
                            return Err(SyntaxError {
                                line: start,
                                code: "syntax_error",
                                message: "unterminated text: a `\"` is missing".into(),
                            });
                        }
                        Some(c) => word.push(c),
                    }
                }
                tokens.push((Token::Text(word, true), start));
            }
            _ => {
                flush(&mut current, &mut tokens, line);
                match character {
                    '{' | '(' => tokens.push((Token::Open, line)),
                    '}' | ')' => tokens.push((Token::Close, line)),
                    '=' | ':' => tokens.push((Token::Assign, line)),
                    '\n' => line += 1,
                    _ => {}
                }
            }
        }
    }
    flush(&mut current, &mut tokens, line);
    Ok(tokens)
}

/// A bare or quoted text: a number when it is unquoted and reads as one.
pub(super) fn scalar(text: String, quoted: bool) -> Value {
    if !quoted && let Ok(number) = text.parse::<f64>() {
        return Value::Number(number);
    }
    Value::Word(text)
}
