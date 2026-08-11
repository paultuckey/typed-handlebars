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
/// This error type provides detailed information about parsing errors,
/// including the location and context of the error.
#[derive(Debug)]
pub struct ParseError {
    pub(crate) message: String,
}

/// Returns the last 32 characters of a string for error context
pub(crate) fn rcap(src: &str) -> &str {
    static CAP_AT: usize = 32;

    if src.len() > CAP_AT {
        &src[src.len() - CAP_AT..]
    } else {
        src
    }
}

impl ParseError {
    /// Creates a new parse error with context from an expression
    pub(crate) fn new(message: &str, expression: &Expression<'_>) -> Self {
        Self {
            message: format!("{} near \"{}\"", message, expression.around()),
        }
    }

    /// Creates a parse error that isn't tied to a particular expression
    ///
    /// Messages built this way should still read in Handlebars terms — naming the template
    /// construct rather than anything about the generated Rust.
    pub(crate) fn general(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }

    /// Creates an error for unclosed blocks
    pub(crate) fn unclosed(preffix: &str) -> Self {
        Self {
            message: format!("unclosed block near {}", rcap(preffix)),
        }
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
        }
    }
}

impl Error for ParseError {}

/// Result type for template parsing operations
pub type Result<T> = std::result::Result<T, ParseError>;
