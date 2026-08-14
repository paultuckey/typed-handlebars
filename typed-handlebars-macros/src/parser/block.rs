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

//! Handlebars block parsing and compilation
//!
//! This module provides functionality for parsing and compiling Handlebars block helpers.
//! It supports various block types including:
//! - `if`/`unless` for conditional rendering
//! - `with` for changing context
//! - `each` for iterating over collections
//!
//! # Block Types
//!
//! ## Conditional Blocks
//! - `{{#if value}}...{{/if}}` - Renders content if value is truthy
//! - `{{#unless value}}...{{/unless}}` - Renders content if value is falsy
//! - `{{else if other}}` - Chains onto either, compiling to a Rust `else if`. See
//!   [`else_branch`](super::else_branch).
//!
//! ## Context Blocks
//! - `{{#with value as item}}...{{/with}}` - Changes context to value
//!
//! ## Iteration Blocks
//! - `{{#each items as item}}...{{/each}}` - Iterates over collection
//! - Supports `@index`, `@first` and `@last`, all answered from one iteration counter that is
//!   only declared when the body actually reads one of them
//! - Supports `else` block for empty collections
//!
//! # Examples
//!
//! ```ignore
//! use block::{Block, BlockFactory};
//! use expression::{Expression, ExpressionType};
//!
//! let template = "{{#if user}}Hello {{user.name}}!{{/if}}";
//! let expr = Expression::from(template).unwrap().unwrap();
//! assert_eq!(expr.expression_type, ExpressionType::Open);
//! ```

use crate::parser::{
    compiler::{Block, BlockFactory, BlockMap, Compile, Local, Rust, append_with_depth},
    else_branch::{self, ElseBranch},
    error::{ParseError, Result},
    expression::{Expression, ExpressionType},
    expression_tokenizer::Token,
};

/// Strips pipe characters from a token value
fn strip_pipes<'a>(token: Token<'a>, expression: &Expression<'a>) -> Result<&'a str> {
    loop {
        return match token.next()? {
            Some(token) => {
                if token.value == "|" {
                    continue;
                }
                Ok(token.value.trim_matches('|'))
            }
            None => Err(ParseError::new("expected variable after as", expression)),
        };
    }
}

/// Reads a local variable declaration from a token
fn read_local<'a>(token: &Token<'a>, expression: &Expression<'a>) -> Result<Local> {
    match token.next()? {
        Some(token) => match token.value {
            "as" => Ok(Local::As(strip_pipes(token, expression)?.to_string())),
            token => Err(ParseError::new(
                &format!("unexpected token {}", token),
                expression,
            )),
        },
        None => Ok(Local::This),
    }
}

/// Handles if/unless block compilation
struct IfOrUnless {}

impl IfOrUnless {
    /// Creates a new if/unless block
    pub fn new<'a>(
        label: &str,
        prefix: &str,
        compile: &'a Compile<'a>,
        token: Token<'a>,
        expression: &'a Expression<'a>,
        rust: &mut Rust,
    ) -> Result<IfOrUnless> {
        match token.next()? {
            Some(var) => {
                // Handlebars truthiness, not a bare `if`: absent, false, "", 0 and an empty list
                // are all falsy, so `{{#if title}}{{title}}{{/if}}` works on the string it prints.
                rust.code.push_str(prefix);
                rust.code.push_str(compile.runtime);
                rust.code.push_str("::Truthy::is_truthy(&");
                compile.write_var(expression, rust, &var)?;
                rust.code.push_str("){");
                Ok(Self {})
            }
            None => Err(ParseError::new(
                &format!("expected variable after {}", label),
                expression,
            )),
        }
    }
}

impl Block for IfOrUnless {
    /// Handles else block compilation
    fn handle_else<'a>(&self, _expression: &'a Expression<'a>, rust: &mut Rust) -> Result<()> {
        rust.code.push_str("}else{");
        Ok(())
    }

    /// `{{#if}}` and `{{#unless}}` are the two blocks an `{{else if}}` can chain onto — their
    /// alternative is a plain `else` and their close is a single `}`, which is what lets a chain of
    /// any length share it.
    fn allows_else_if(&self) -> bool {
        true
    }
}

/// Factory for if blocks
struct IfFty {}

impl BlockFactory for IfFty {
    /// Opens an if block
    fn open<'a>(
        &self,
        compile: &'a Compile<'a>,
        token: Token<'a>,
        expression: &'a Expression<'a>,
        rust: &mut Rust,
    ) -> Result<Box<dyn Block>> {
        Ok(Box::new(IfOrUnless::new(
            "if", "if ", compile, token, expression, rust,
        )?))
    }
}

/// Factory for unless blocks
struct UnlessFty {}

impl BlockFactory for UnlessFty {
    /// Opens an unless block
    fn open<'a>(
        &self,
        compile: &'a Compile<'a>,
        token: Token<'a>,
        expression: &'a Expression<'a>,
        rust: &mut Rust,
    ) -> Result<Box<dyn Block>> {
        Ok(Box::new(IfOrUnless::new(
            "unless", "if !", compile, token, expression, rust,
        )?))
    }
}

/// Handles with block compilation
struct With {
    local: Local,
}

impl With {
    /// Creates a new with block
    pub fn new<'a>(
        by_ref: bool,
        compile: &'a Compile<'a>,
        token: Token<'a>,
        expression: &'a Expression<'a>,
        rust: &mut Rust,
    ) -> Result<Self> {
        let next = token.next()?.ok_or_else(|| {
            ParseError::new(
                &format!(
                    "expected variable after with{}",
                    if by_ref { "_ref" } else { "" }
                ),
                expression,
            )
        })?;
        let local = read_local(&next, expression)?;
        rust.code.push_str("{let ");
        compile.write_local(&mut rust.code, &local);
        rust.code.push_str(" = ");
        if by_ref {
            rust.code.push('&');
        }
        compile.write_var(expression, rust, &next)?;
        rust.code.push(';');
        Ok(Self { local })
    }
}

impl Block for With {
    /// Returns the local variable
    fn local<'a>(&self) -> &Local {
        &self.local
    }
}

/// Factory for with blocks
struct WithFty {}

impl BlockFactory for WithFty {
    /// Opens a with block
    fn open<'a>(
        &self,
        compile: &'a Compile<'a>,
        token: Token<'a>,
        expression: &'a Expression<'a>,
        rust: &mut Rust,
    ) -> Result<Box<dyn Block>> {
        Ok(Box::new(With::new(true, compile, token, expression, rust)?))
    }
}

/// Handles each block compilation
struct Each {
    local: Local,
    /// The iteration counter, when the body reads `@index`, `@first` or `@last`.
    counter: Option<String>,
    /// The list's length, when the body reads `@last` and so has to know where the end is.
    length: Option<String>,
    has_else: bool,
}

/// The `@…` variables a `{{#each}}` body reads from the block itself.
///
/// Scanning for these before the loop is emitted is what lets the counter be declared only when
/// something actually reads it, so a plain `{{#each}}` compiles to a plain `for`.
#[derive(Clone, Copy, Default)]
pub(super) struct Privates {
    /// `@index`, `@first` or `@last` — all three are answered from the iteration counter.
    counter: bool,
    /// `@last`, which additionally compares the counter against the list's length.
    length: bool,
}

impl Privates {
    /// Records what one expression reads, `out` levels out from the block being scanned.
    fn absorb(&mut self, content: &str, out: i32) {
        self.counter |= reads(content, "index", out) || reads(content, "first", out);
        if reads(content, "last", out) {
            self.counter = true;
            self.length = true;
        }
    }
}

/// Whether `content` reads `@name` belonging to the block `out` levels outwards.
///
/// `{{@../index}}` inside a nested block means the *enclosing* block's counter, so each `../` steps
/// one level out and only a reference that lands on zero belongs to the block being scanned.
fn reads(content: &str, name: &str, mut out: i32) -> bool {
    let Some(at) = content.find(name) else {
        return false;
    };
    let Some(start) = content[..at].rfind('@') else {
        return false;
    };
    let mut prefix = &content[start + 1..at];
    while let Some(rest) = prefix.strip_prefix("../") {
        out -= 1;
        prefix = rest;
    }
    // Nothing but `../` may sit between the `@` and the name, or `{{#each xs as |first|}}` would
    // read as a reference to `@first`.
    prefix.is_empty() && out == 0
}

/// Scans a block's body for the `@…` variables it needs this block to supply.
///
/// Stops at the matching close, so a nested loop's `{{@index}}` counts against that loop rather
/// than this one.
///
/// **Nesting is counted in loops, not in blocks.** `{{#if}}`, `{{#unless}}` and `{{#with}}` are
/// transparent to an `@…` lookup, so a reference inside one still belongs to this block — which is
/// why the two counters below are not the same number. [`Compile::find_private_scope`] resolves
/// references the same way, and the two have to agree: this one decides whether the counter is
/// declared, that one decides what reads it.
pub(super) fn check_for_privates(src: &str) -> Result<Privates> {
    let mut found = Privates::default();
    // Whether each still-open nested block was a loop, so a close knows what it ends.
    let mut opened: Vec<bool> = Vec::new();
    // How many loops deep the cursor is, relative to the block being scanned.
    let mut loops = 0;
    let mut exp = Expression::from(src)?;
    while let Some(expr) = &exp {
        match expr.expression_type {
            // A comment's text and a raw block's body are literal, so nothing in them refers to
            // anything. Falling through rather than `continue`-ing is load-bearing: this loop
            // advances at the bottom, so a `continue` here never terminates — a comment inside an
            // `{{#each}}` used to hang the compiler outright.
            ExpressionType::Comment | ExpressionType::Escaped => {}
            ExpressionType::Open => {
                // A block's opening expression is evaluated in the scope *around* it, which is why
                // `{{#if @first}}` reads this block's counter rather than the `{{#if}}`'s.
                found.absorb(expr.content, loops);
                let is_loop = opens_a_loop(expr.content)?;
                opened.push(is_loop);
                if is_loop {
                    loops += 1;
                }
            }
            ExpressionType::Close => match opened.pop() {
                Some(was_loop) => {
                    if was_loop {
                        loops -= 1;
                    }
                }
                // Nothing left to close but the block being scanned.
                None => return Ok(found),
            },
            _ => found.absorb(expr.content, loops),
        }
        exp = expr.next()?;
    }
    Ok(found)
}

/// Whether an opening expression opens an `{{#each}}` — the only block an `@…` can come from.
fn opens_a_loop(content: &str) -> Result<bool> {
    Ok(Token::first(content)?.is_some_and(|head| head.value == "each"))
}

/// Checks if a block contains an else block
fn check_for_else(src: &str) -> Result<bool> {
    let mut exp = Expression::from(src)?;
    let mut depth = 1;
    while let Some(expr) = &exp {
        match expr.expression_type {
            // As in `check_for_privates`: fall through so the loop advances. A `continue` here
            // never terminates.
            ExpressionType::Comment | ExpressionType::Escaped => {}
            ExpressionType::Open => depth += 1,
            ExpressionType::Close => {
                depth -= 1;
                if depth == 0 {
                    return Ok(false);
                }
            }
            _ => {
                // Only a plain `{{else}}` — a chain cannot open on an `{{#each}}` anyway, and the
                // classifier is what makes `{{ else }}` with spaces count here too.
                if depth == 1 && else_branch::classify(expr.content) == Some(ElseBranch::Plain) {
                    return Ok(true);
                }
            }
        }
        exp = expr.next()?;
    }
    Ok(false)
}

impl Each {
    /// Creates a new each block
    pub fn new<'a>(
        compile: &'a Compile<'a>,
        token: Token<'a>,
        expression: &'a Expression<'a>,
        rust: &mut Rust,
    ) -> Result<Self> {
        let next = match token.next()? {
            Some(next) => next,
            None => {
                return Err(ParseError::new("expected variable after each", expression));
            }
        };
        let privates = check_for_privates(expression.postfix)?;
        // The `__th_` prefix keeps these clear of the locals a template can name. A block alias
        // goes through `sanitise_ident` and lands on `<name>_<depth>`, so a counter called `i_0`
        // was shadowed by a plain `{{#each xs as |i|}}` — and the loop then tried to increment the
        // item instead of the counter, which the author saw as `E0368` against their template.
        let scope = compile.open_stack.len();
        let counter = privates.counter.then(|| format!("__th_i_{scope}"));
        let length = privates.length.then(|| format!("__th_n_{scope}"));

        let local = read_local(&next, expression)?;
        let has_else = check_for_else(expression.postfix)?;

        // `@last` is "the counter has reached the end", so the length has to be taken before the
        // loop starts. `as_ref()` is the same borrow the loop takes, and costs nothing.
        if let Some(length) = &length {
            rust.code.push_str("let ");
            rust.code.push_str(length);
            rust.code.push_str(" = ");
            compile.write_var(expression, rust, &next)?;
            rust.code.push_str(".as_ref().len();");
        }
        if let Some(counter) = &counter {
            rust.code.push_str("let mut ");
            rust.code.push_str(counter);
            // Typed, so that comparing it against a length is not left to inference.
            rust.code.push_str(" = 0usize;");
        }
        if has_else {
            rust.code.push_str("{let mut empty = true;");
        }
        rust.code.push_str("for ");
        compile.write_local(&mut rust.code, &local);
        rust.code.push_str(" in ");
        compile.write_var(expression, rust, &next)?;
        // The field's only `AsRef` bound is the generated one, so this is unambiguous.
        rust.code.push_str(".as_ref()");
        rust.code.push('{');
        if has_else {
            rust.code.push_str("empty = false;");
        }
        Ok(Self {
            local,
            counter,
            length,
            has_else,
        })
    }

    /// The counter's name, or an error if the scan that declares it disagreed with this
    /// resolution.
    ///
    /// [`check_for_privates`] and the compiler walk the template separately and have to agree about
    /// which block supplies an `@…` variable — the same hazard `else_branch` exists for. Reaching
    /// here means they did not, so it reports against the template rather than unwrapping into a
    /// proc-macro panic.
    fn counter(&self, expression: &Expression<'_>, name: &str) -> Result<&str> {
        self.counter.as_deref().ok_or_else(|| {
            ParseError::new(&format!("`@{}` is not available here", name), expression)
        })
    }
    /// Writes a map variable access
    fn write_map_var(&self, depth: usize, suffix: &str, rust: &mut Rust) {
        append_with_depth(
            depth,
            if let Local::As(name) = &self.local {
                name.as_str()
            } else {
                "this"
            },
            &mut rust.code,
        );
        rust.code.push_str(suffix)
    }

    /// Writes an indexer increment
    fn write_indexer(&self, rust: &mut Rust) {
        if let Some(counter) = &self.counter {
            rust.code.push_str(counter);
            rust.code.push_str("+=1;");
        }
    }
}

impl Block for Each {
    fn handle_else<'a>(&self, _expression: &'a Expression<'a>, rust: &mut Rust) -> Result<()> {
        self.write_indexer(rust);
        rust.code.push_str("} if empty {");
        Ok(())
    }

    fn resolve_private<'a>(
        &self,
        depth: usize,
        expression: &'a Expression<'a>,
        name: &str,
        rust: &mut Rust,
    ) -> Result<()> {
        match name {
            "index" => rust.code.push_str(self.counter(expression, name)?),
            // `@first` and `@last` are parenthesised because they land inside a `&…` — the
            // escaper's argument, or `Truthy::is_truthy`'s. Without them `&__th_i_0 == 0` would
            // take a reference to the counter and compare *that*.
            "first" => {
                rust.code.push('(');
                rust.code.push_str(self.counter(expression, name)?);
                rust.code.push_str(" == 0)");
            }
            "last" => {
                rust.code.push('(');
                rust.code.push_str(self.counter(expression, name)?);
                rust.code.push_str(" + 1 == ");
                rust.code.push_str(
                    self.length.as_deref().ok_or_else(|| {
                        ParseError::new("`@last` is not available here", expression)
                    })?,
                );
                rust.code.push(')');
            }
            "key" => self.write_map_var(depth, ".0", rust),
            "value" => self.write_map_var(depth, ".1", rust),
            _ => {
                return Err(ParseError::new(
                    &format!("unexpected variable {}", name),
                    expression,
                ));
            }
        }
        Ok(())
    }

    fn handle_close<'a>(&self, rust: &mut Rust) {
        if self.has_else {
            rust.code.push_str("}}");
        } else {
            self.write_indexer(rust);
            rust.code.push('}');
        }
    }

    fn local<'a>(&self) -> &Local {
        &self.local
    }

    /// `{{#each}}` is the only block an `@…` comes from — see
    /// [`Compile::find_private_scope`](super::compiler::Compile).
    fn supplies_privates(&self) -> bool {
        true
    }
}

/// Factory for each blocks
struct EachFty {}

impl BlockFactory for EachFty {
    /// Opens an each block
    fn open<'a>(
        &self,
        compile: &'a Compile<'a>,
        token: Token<'a>,
        expression: &'a Expression<'a>,
        rust: &mut Rust,
    ) -> Result<Box<dyn Block>> {
        Ok(Box::new(Each::new(compile, token, expression, rust)?))
    }
}

const IF: IfFty = IfFty {};
const UNLESS: UnlessFty = UnlessFty {};
const WITH: WithFty = WithFty {};
const EACH: EachFty = EachFty {};

/// Adds built-in block helpers to the block map
pub fn add_builtins(map: &mut BlockMap) {
    map.insert("if", &IF);
    map.insert("unless", &UNLESS);
    map.insert("with", &WITH);
    map.insert("each", &EACH);
}
