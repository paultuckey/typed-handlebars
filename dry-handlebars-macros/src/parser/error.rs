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

//! Error handling for the Handlebars parser
//!
//! This module provides error types and handling for the template parsing process.
//! It includes detailed error messages with context about where parsing errors occurred.

use crate::parser::expression::Expression;
use std::{error::Error, fmt::Display};

/// Error type for template parsing failures
///
/// Carries where in the template the problem is, so the developer — who may not read Rust — gets a
/// line and column in their `.hbs` file rather than a proc-macro panic.
#[derive(Debug)]
pub struct ParseError {
    pub(crate) message: String,
    /// Address of the offending text.
    ///
    /// Every slice the parser handles borrows from the one template string, so recording the
    /// address here lets [`ParseError::offset_in`] recover a byte offset later, at the point where
    /// the whole template is in scope. Only ever compared and subtracted, never dereferenced.
    at: Option<usize>,
}

impl ParseError {
    /// Creates a new parse error pointing at an expression
    pub(crate) fn new(message: &str, expression: &Expression<'_>) -> Self {
        Self {
            message: message.to_string(),
            at: Some(expression.raw.as_ptr() as usize),
        }
    }

    /// Creates a parse error that isn't tied to a particular expression
    ///
    /// Messages built this way should still read in Handlebars terms — naming the template
    /// construct rather than anything about the generated Rust.
    pub(crate) fn general(message: &str) -> Self {
        Self {
            message: message.to_string(),
            at: None,
        }
    }

    /// Creates an error for unclosed blocks
    pub(crate) fn unclosed(preffix: &str) -> Self {
        Self {
            message: "unclosed block — every {{#…}} needs a matching {{/…}}".to_string(),
            // The block opened somewhere after this point; the end of the text before it is the
            // closest position we have.
            at: Some(preffix.as_ptr() as usize + preffix.len()),
        }
    }

    /// Attaches a position, if the error does not already have one.
    pub(crate) fn or_at(mut self, text: &str) -> Self {
        self.at.get_or_insert(text.as_ptr() as usize);
        self
    }

    /// The byte offset of this error within `src`, when it came from `src`.
    pub(crate) fn offset_in(&self, src: &str) -> Option<usize> {
        let start = src.as_ptr() as usize;
        let at = self.at?;
        (at >= start && at <= start + src.len()).then(|| at - start)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        Self {
            message: err.to_string(),
            at: None,
        }
    }
}

impl Error for ParseError {}

/// Result type for template parsing operations
pub type Result<T> = std::result::Result<T, ParseError>;
