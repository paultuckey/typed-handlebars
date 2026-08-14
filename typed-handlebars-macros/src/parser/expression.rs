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

//! Handlebars expression parsing
//!
//! This module provides functionality for parsing Handlebars expressions from template strings.
//! It handles various types of expressions including variables, blocks, comments, and escaped content.
//!
//! # Expression Types
//!
//! The module supports the following types of expressions:
//! - Variables: `{{name}}`
//! - HTML-escaped variables: `{{{name}}}`
//! - Block helpers: `{{#helper}}...{{/helper}}`
//! - Comments: `{{! comment }}` or `{{!-- comment --}}`
//! - Escaped content: `\{{name}}` or `{{{{name}}}}this bit here is not parsed {{not_interpolated}} and output raw{{{{/name}}}}`
//!
//! # Examples
//!
//! ```ignore
//! use expression::{Expression, ExpressionType};
//!
//! let template = "Hello {{name}}!";
//! let expr = Expression::from(template).unwrap().unwrap();
//! assert_eq!(expr.expression_type, ExpressionType::HtmlEscaped);
//! assert_eq!(expr.content, "name");
//! ```

use std::fmt::Display;

use crate::parser::error::{ParseError, Result};

/// Types of Handlebars expressions
#[derive(Debug, Clone, Copy)]
pub enum ExpressionType {
    /// Comment expression: `{{! comment }}`
    Comment,
    HtmlEscaped,
    Raw,
    Open,
    Close,
    Escaped,
}

/// Represents a parsed Handlebars expression
#[derive(Debug, Clone, Copy)]
pub struct Expression<'a> {
    /// The type of expression
    pub expression_type: ExpressionType,
    /// Text before the expression
    pub prefix: &'a str,
    /// The expression content
    pub content: &'a str,
    /// Text after the expression
    pub postfix: &'a str,
    /// The complete expression including delimiters
    pub raw: &'a str,
}

/// Safely extracts a substring of specified length
#[inline]
fn nibble(src: &str, start: usize, len: usize) -> Result<usize> {
    let end = start + len;
    if end >= src.len() {
        return Err(ParseError::unclosed(src));
    }
    Ok(end)
}

impl<'a> Expression<'a> {
    /// Creates a new expression by finding its closing delimiter
    fn close(
        expression_type: ExpressionType,
        preffix: &'a str,
        start: &'a str,
        end: &'static str,
    ) -> Result<Self> {
        match start.find(end) {
            Some(mut pos) => {
                if pos == 0 {
                    return Err(
                        ParseError::general("empty expression: `{{}}` has no name in it")
                            .or_at(start),
                    );
                }
                let mut postfix = &start[pos + end.len()..];
                if &start[pos - 1..pos] == "~" {
                    postfix = postfix.trim_start();
                    pos -= 1;
                }
                Ok(Self {
                    expression_type,
                    prefix: preffix,
                    content: &start[..pos],
                    postfix,
                    raw: &start[..pos + end.len()],
                })
            }
            None => Err(ParseError::unclosed(preffix)),
        }
    }

    /// Parses a comment expression.
    ///
    /// A comment scans for its own close rather than delegating to [`Self::close`], for two
    /// reasons.
    ///
    /// **The trimming close puts the `~` inside the token.** `{{! … ~}}` and `{{!-- … --~}}` both
    /// trim the whitespace after the comment, but `close` only recognises a `~` sitting immediately
    /// *before* the token it was given — so `--~}}` is not a match for `--}}` at all, and a long
    /// comment closed that way used to read as unclosed.
    ///
    /// **A comment has no name.** Every other expression needs one, so `close` rejects an empty
    /// one; `{{!}}` and `{{!----}}` are valid comments, as they are in handlebars.js.
    ///
    /// Whichever close comes first wins, also as in handlebars.js: `{{!-- a --}} b --~}}` ends at
    /// the `--}}` and leaves ` b --~}}` as text. A trimming token starts one byte before the plain
    /// token it would otherwise be confused with, so "first wins" prefers it with no special case,
    /// and the two can never start at the same byte.
    fn check_comment(preffix: &'a str, start: &'a str) -> Result<Self> {
        // A long comment may contain `}}`, which is the whole point of the form.
        let long = start.starts_with("--");
        let body = if long { &start[2..] } else { start };
        let (plain, trimming) = if long {
            ("--}}", "--~}}")
        } else {
            ("}}", "~}}")
        };

        let close = [(plain, false), (trimming, true)]
            .into_iter()
            .filter_map(|(end, trims)| body.find(end).map(|at| (at, end, trims)))
            .min_by_key(|(at, ..)| *at);

        let Some((at, end, trims)) = close else {
            return Err(if long {
                // Saying "unclosed block" here sends people looking for a missing `{{/…}}`.
                ParseError::general(
                    "unclosed comment — a `{{!-- … }}` comment has to end with `--}}` or `--~}}`",
                )
                .or_at(start)
            } else {
                ParseError::unclosed(preffix)
            });
        };

        let mut postfix = &body[at + end.len()..];
        if trims {
            postfix = postfix.trim_start();
        }
        Ok(Self {
            expression_type: ExpressionType::Comment,
            prefix: preffix,
            content: &body[..at],
            postfix,
            raw: &body[..at + end.len()],
        })
    }

    /// Finds the closing delimiter for an escaped expression
    fn find_closing_escape(open: Expression<'a>) -> Result<Self> {
        let mut postfix = open.postfix;
        let mut from: usize = 0;
        loop {
            let candidate = postfix
                .find("{{{{/")
                .ok_or(ParseError::unclosed(open.raw))?;
            let start = candidate + 5;
            let remains = &postfix[start..];
            let close = remains.find("}}}}").ok_or(ParseError::unclosed(open.raw))?;
            let end = start + close + 4;
            if &remains[..close] == open.content {
                return Ok(Self {
                    expression_type: ExpressionType::Escaped,
                    prefix: open.prefix,
                    content: &open.postfix[..from + candidate],
                    postfix: &postfix[end..],
                    raw: open.raw,
                });
            }
            from += end;
            postfix = &postfix[from..];
        }
    }

    /// Parses the next expression from a template string
    pub fn from(src: &'a str) -> Result<Option<Self>> {
        match src.find("{{") {
            Some(start) => {
                let mut second = nibble(src, start, 3)?;
                if start > 0 && &src[start - 1..start] == "\\" {
                    return Ok(Some(Self::close(
                        ExpressionType::Escaped,
                        &src[..start - 1],
                        &src[second - 1..],
                        "}}",
                    )?));
                }
                let mut prefix = &src[..start];
                let mut marker = &src[start + 2..second];
                if marker == "~" {
                    prefix = prefix.trim_end();
                    second = nibble(src, second, 1)?;
                    marker = &src[start + 3..second];
                }
                Ok(Some(match marker {
                    "{" => {
                        let next = nibble(src, second, 1)?;
                        let char = &src[second..next];
                        if char == "{" {
                            second = next;
                            let next = nibble(src, second, 1)?;
                            if &src[second..next] == "~" {
                                second = next;
                                prefix = prefix.trim_end();
                            }
                            return Ok(Some(Self::find_closing_escape(Self::close(
                                ExpressionType::Escaped,
                                prefix,
                                &src[second..],
                                "}}}}",
                            )?)?));
                        }
                        if char == "~" {
                            second = next;
                            prefix = prefix.trim_end();
                        }
                        Self::close(ExpressionType::Raw, prefix, &src[second..], "}}}")?
                    }
                    "!" => Self::check_comment(prefix, &src[second..])?,
                    "#" => Self::close(ExpressionType::Open, prefix, &src[second..], "}}")?,
                    "/" => Self::close(ExpressionType::Close, prefix, &src[second..], "}}")?,
                    _ => Self::close(
                        ExpressionType::HtmlEscaped,
                        prefix,
                        &src[second - 1..],
                        "}}",
                    )?,
                }))
            }
            None => Ok(None),
        }
    }

    /// Parses the next expression after this one
    pub fn next(&self) -> Result<Option<Self>> {
        Self::from(self.postfix)
    }
}

impl<'a> Display for Expression<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.raw)
    }
}
