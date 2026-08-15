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
    /// Whether this expression was alone on its line and so consumed it.
    ///
    /// Only meaningful to [`Expression::next`], which needs it to know whether the text it is about
    /// to read begins at the start of a line. See [`Expression::apply_standalone`].
    pub standalone: bool,
}

/// Horizontal whitespace — the run that may sit around a tag on its own line without disqualifying
/// it. A newline ends the line rather than padding it, so it is not in here.
const BLANK: [char; 2] = [' ', '\t'];

/// The prefix with a standalone tag's indentation removed, or `None` if the tag is not at the start
/// of its line.
///
/// `at_line_start` covers the case the text cannot: a prefix with no newline in it is at the start
/// of a line only when nothing has been emitted on that line yet — at the very beginning of the
/// template, or straight after another standalone tag consumed its newline.
fn line_start(prefix: &str, at_line_start: bool) -> Option<&str> {
    let indent = match prefix.rfind('\n') {
        Some(newline) => &prefix[newline + 1..],
        None if at_line_start => prefix,
        None => return None,
    };
    indent
        .chars()
        .all(|c| BLANK.contains(&c))
        .then(|| &prefix[..prefix.len() - indent.len()])
}

/// The postfix with a standalone tag's trailing newline removed, or `None` if anything but
/// whitespace follows the tag on its line.
///
/// The end of the template ends a line too, so a tag with nothing after it stands alone.
fn line_end(postfix: &str) -> Option<&str> {
    let rest = postfix.trim_start_matches(BLANK);
    // `\r\n` goes as one, or the `\r` would be left behind as stray output.
    rest.strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))
        .or(rest.is_empty().then_some(rest))
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
                    standalone: false,
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
            standalone: false,
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
                    standalone: false,
                });
            }
            from += end;
            postfix = &postfix[from..];
        }
    }

    /// Parses the first expression in a template string.
    ///
    /// The start of a template is the start of a line, which is what lets a tag on the very first
    /// line stand alone with nothing before it.
    pub fn from(src: &'a str) -> Result<Option<Self>> {
        Self::parse(src, true)
    }

    /// Parses the next expression, given whether the text about to be read begins a line.
    fn parse(src: &'a str, at_line_start: bool) -> Result<Option<Self>> {
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
                let mut expression = match marker {
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
                };
                expression.apply_standalone(at_line_start);
                Ok(Some(expression))
            }
            None => Ok(None),
        }
    }

    /// Parses the next expression after this one
    pub fn next(&self) -> Result<Option<Self>> {
        // A standalone tag took its own newline with it, so whatever follows starts a fresh line.
        // Anything else leaves the cursor mid-line, even when the text between is only spaces.
        Self::parse(self.postfix, self.standalone)
    }

    /// Removes the whitespace around a tag that is alone on its line, as handlebars.js does.
    ///
    /// A line whose only content is a block tag, an `{{else}}`, or a comment contributes nothing of
    /// its own to the output: both its indentation and its trailing newline go, so the tag leaves
    /// no trace. Without this a template laid out over several lines gains a blank line after every
    /// tag — which is most templates.
    ///
    /// An interpolation is **not** standalone: `{{ name }}` alone on a line keeps its newline,
    /// because it is there to produce output. Checked against handlebars.js, along with the rest of
    /// the rule.
    ///
    /// Nothing is rewritten here, only narrowed — `prefix` and `postfix` stay subslices of the
    /// original template, so the byte offsets error positions are built from stay valid.
    fn apply_standalone(&mut self, at_line_start: bool) {
        if !self.can_stand_alone() {
            return;
        }
        if let (Some(prefix), Some(postfix)) = (
            line_start(self.prefix, at_line_start),
            line_end(self.postfix),
        ) {
            self.prefix = prefix;
            self.postfix = postfix;
            self.standalone = true;
        }
    }

    /// Whether this kind of expression may stand alone on a line.
    ///
    /// Raw blocks are left out deliberately: their content passes through here where handlebars.js
    /// hands it to a helper, which is a divergence of its own, so there is no agreed answer to
    /// match.
    fn can_stand_alone(&self) -> bool {
        match self.expression_type {
            ExpressionType::Open | ExpressionType::Close | ExpressionType::Comment => true,
            // `{{else}}` and `{{else if …}}` arrive as ordinary interpolations but are block tags.
            ExpressionType::HtmlEscaped => super::else_branch::classify(self.content).is_some(),
            ExpressionType::Raw | ExpressionType::Escaped => false,
        }
    }
}

impl<'a> Display for Expression<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two halves of the standalone rule, tested directly: they are pure string functions, and
    /// testing them here covers cases the end-to-end suites cannot reach — `\r\n` among them, since
    /// a carriage return does not survive code generation today.
    #[test]
    fn a_line_start_is_a_newline_followed_by_blanks() {
        assert_eq!(line_start("a\n    ", false), Some("a\n"));
        assert_eq!(line_start("a\n\t", false), Some("a\n"));
        assert_eq!(line_start("a\n", false), Some("a\n"));
        assert_eq!(line_start("a\r\n  ", false), Some("a\r\n"));
        // Anything but blanks after the newline means the tag is not alone.
        assert_eq!(line_start("a\n  x", false), None);
        assert_eq!(line_start("a\nx  ", false), None);
    }

    /// With no newline in the prefix the text cannot say whether a line has begun, so the caller's
    /// flag decides — true at the start of the template, or straight after a standalone tag.
    #[test]
    fn a_prefix_with_no_newline_defers_to_the_caller() {
        assert_eq!(line_start("  ", true), Some(""));
        assert_eq!(line_start("", true), Some(""));
        assert_eq!(line_start("  ", false), None);
        assert_eq!(line_start("", false), None);
        // Even at a line start, real content on the line disqualifies it.
        assert_eq!(line_start("  x", true), None);
    }

    #[test]
    fn a_line_end_takes_the_blanks_and_one_newline() {
        assert_eq!(line_end("\nb"), Some("b"));
        assert_eq!(line_end("   \nb"), Some("b"));
        assert_eq!(line_end("\t\nb"), Some("b"));
        // `\r\n` goes as a unit, or the `\r` would be left behind as output.
        assert_eq!(line_end("\r\nb"), Some("b"));
        assert_eq!(line_end("  \r\nb"), Some("b"));
        // Exactly one newline: a blank line after the tag is the author's, not the tag's.
        assert_eq!(line_end("\n\nb"), Some("\nb"));
        // The end of the template ends a line too.
        assert_eq!(line_end(""), Some(""));
        assert_eq!(line_end("   "), Some(""));
        // Anything else on the line and the tag is not alone.
        assert_eq!(line_end(" x\n"), None);
        assert_eq!(line_end("x"), None);
    }
}
