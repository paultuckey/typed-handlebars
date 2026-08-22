// MIT License
//
// Copyright (c) 2024 Jerome Johnson
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Handlebars expression tokenization
//!
//! This module provides functionality for tokenizing Handlebars expressions into their component parts.
//! It handles various token types including:
//! - Literals: Plain text values
//! - Private variables: Variables prefixed with @ (e.g. @index)
//! - Sub-expressions: Parenthesized expressions
//!
//! # Token Types
//!
//! ## Literals
//! Plain text values that are not special tokens:
//! ```handlebars
//! name
//! user.age
//! ```
//!
//! ## Private Variables
//! Variables prefixed with @ that have special meaning:
//! ```handlebars
//! @index
//! @key
//! @value
//! ```
//!
//! ## Sub-expressions
//! Parenthesized expressions that are evaluated first:
//! ```handlebars
//! (helper arg1 arg2)
//! (math.add 1 2)
//! ```
//!
//! # Examples
//!
//! ```ignore
//! use expression_tokenizer::{Token, TokenType};
//!
//! let src = "user.name (helper arg) @index";
//! let token = Token::first(src).unwrap().unwrap();
//! assert_eq!(token.value, "user.name");
//! assert_eq!(token.token_type, TokenType::Literal);
//! ```

use crate::parser::error::{ParseError, Result};

/// Types of tokens that can be parsed from an expression
#[derive(Clone)]
pub enum TokenType<'a> {
    /// A parenthesized sub-expression
    SubExpression(&'a str),
    /// A private variable prefixed with @
    PrivateVariable,
    Variable,
    /// A plain text literal
    Literal,
}

/// A token parsed from an expression
#[derive(Clone)]
pub struct Token<'a> {
    /// The type of token
    pub token_type: TokenType<'a>,
    /// The token's value
    pub value: &'a str,
    /// The remaining text after this token
    pub tail: &'a str,
}

/// Finds the closing parenthesis for a sub-expression
fn find_closing(src: &str) -> Result<usize> {
    let mut count = 1;
    let rest = &src[1..];
    for (i, c) in rest.char_indices() {
        match c {
            '(' => count += 1,
            ')' => count -= 1,
            _ => (),
        }
        if count == 0 {
            return Ok(i + 1);
        }
    }
    Err(ParseError::general("unmatched brackets").or_at(src))
}

fn find_end_of_string(src: &str, quote: char) -> Result<usize> {
    let cliped = &src[1..];
    let mut escaped = false;
    for (i, c) in cliped.char_indices() {
        match c {
            '\\' => escaped = !escaped,
            c if c == quote && !escaped => return Ok(i + 1 + quote.len_utf8()),
            _ => escaped = false,
        }
    }
    Err(ParseError::general("unterminated string").or_at(src))
}

/// The quote a string literal opens with, if this token is one.
///
/// Handlebars accepts both spellings — `{{ t "Save" }}` and `{{ t 'Save' }}` — and a designer has
/// no reason to know that one of them was easier to lex.
fn opening_quote(src: &str) -> Option<char> {
    src.chars().next().filter(|c| *c == '"' || *c == '\'')
}

/// Whether a whole token is a number.
///
/// handlebars.js lexes `-?[0-9]+(\.[0-9]+)?` as a number, but only when the token ends there —
/// which is what keeps `{{2nd}}` a path rather than a malformed number. Even then it is only a
/// *literal* where it is an argument: a lone `{{ 42 }}` is resolved as a path, so `{{ 42 }}`
/// against `{"42": "answer"}` writes `answer` rather than `42`. Position decides, so this is asked
/// where an argument is read rather than while lexing.
fn is_number(src: &str) -> bool {
    let digits = src.strip_prefix('-').unwrap_or(src);
    let (whole, fraction) = match digits.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (digits, None),
    };
    !whole.is_empty()
        && whole.bytes().all(|b| b.is_ascii_digit())
        && fraction.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()))
}

/// Finds the end of a token by looking for whitespace or special characters
fn find_end(src: &str) -> usize {
    for (i, c) in src.char_indices() {
        if " (\n\r\t".contains(c) {
            return i;
        }
    }
    src.len()
}

fn invalid_variable_name(src: &str) -> bool {
    if src.starts_with("../") {
        return false; // ../ is valid for relative paths
    }
    // A digit may start a name: handlebars.js reads `{{2nd}}` as a variable, and a designer has no
    // reason to know that Rust would not. Code generation renames it to something Rust accepts.
    src.chars()
        .next()
        .map(|c| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(false)
}

/// Parses a single token from the input string
fn parse<'a>(src: &'a str) -> Result<Option<Token<'a>>> {
    Ok(match src.chars().next() {
        Some('@') => {
            let end = find_end(src);
            Some(Token {
                token_type: TokenType::PrivateVariable,
                value: &src[1..end],
                tail: src[end..].trim_start(),
            })
        }
        Some('(') => {
            let end = find_closing(src)?;
            Some(Token {
                token_type: TokenType::SubExpression(&src[..end]),
                value: &src[1..end],
                tail: src[end + 1..].trim_start(),
            })
        }
        None => None,
        _ => {
            let (end, token_type) = if let Some(quote) = opening_quote(src) {
                (find_end_of_string(src, quote)?, TokenType::Literal)
            } else {
                let end = find_end(src);
                (
                    end,
                    if invalid_variable_name(src) {
                        TokenType::Literal
                    } else {
                        TokenType::Variable
                    },
                )
            };
            Some(Token {
                token_type,
                value: &src[..end],
                tail: src[end..].trim_start(),
            })
        }
    })
}

impl<'a> Token<'a> {
    /// The text inside a quoted string literal, if this token is one.
    ///
    /// Handlebars accepts both spellings — `{{ t "Save" }}` and `{{ t 'Save' }}` — so both read as
    /// `Save`.
    pub fn quoted_text(&self) -> Option<&'a str> {
        let quote = opening_quote(self.value)?;
        self.value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
    }

    /// A literal's text with any quotes removed, or the token's value unchanged.
    ///
    /// A number reads as the digits the template spelled, which is what lets a helper argument be
    /// handed over as the source text.
    pub fn literal_text(&self) -> &'a str {
        self.quoted_text().unwrap_or(self.value)
    }

    /// Whether this token is a number, and so a literal wherever it is an argument.
    ///
    /// See [`is_number`] for why position rather than lexing decides.
    pub fn numeric(&self) -> bool {
        is_number(self.value)
    }

    /// Parses the first token from a string
    pub fn first(src: &'a str) -> Result<Option<Self>> {
        parse(src.trim())
    }

    /// Parses the next token after this one
    pub fn next(&self) -> Result<Option<Self>> {
        parse(self.tail)
    }
}
