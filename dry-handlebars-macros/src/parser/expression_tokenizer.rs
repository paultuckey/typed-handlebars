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

fn find_end_of_string(src: &str) -> Result<usize> {
    let cliped = &src[1..];
    let mut escaped = false;
    for (i, c) in cliped.char_indices() {
        match c {
            '\\' => escaped = !escaped,
            '"' => {
                if !escaped {
                    return Ok(i + 2);
                }
            }
            _ => (),
        }
    }
    Err(ParseError::general("unterminated string").or_at(src))
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
    src.chars()
        .next()
        .map(|c| !(c.is_alphabetic() || c == '_'))
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
            let (end, token_type) = if src.starts_with('"') {
                (find_end_of_string(src)?, TokenType::Literal)
            } else {
                (
                    find_end(src),
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
    /// Parses the first token from a string
    pub fn first(src: &'a str) -> Result<Option<Self>> {
        parse(src.trim())
    }

    /// Parses the next token after this one
    pub fn next(&self) -> Result<Option<Self>> {
        parse(self.tail)
    }
}
