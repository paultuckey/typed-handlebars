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
//!
//! ## Context Blocks
//! - `{{#with value as item}}...{{/with}}` - Changes context to value
//!
//! ## Iteration Blocks
//! - `{{#each items as item}}...{{/each}}` - Iterates over collection
//! - Supports `@index` for accessing current index
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
                rust.code.push_str("::dry_handlebars::Truthy::is_truthy(&");
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
    indexer: Option<String>,
    has_else: bool,
}

/// Checks if a string contains an indexer expression at the given depth
fn contains_indexer(src: &str, mut depth: i32) -> bool {
    match src.find("index") {
        Some(pos) => match src[..pos].rfind('@') {
            Some(start) => {
                let mut prefix = &src[start + 1..pos];
                while prefix.starts_with("../") {
                    depth -= 1;
                    prefix = &prefix[3..];
                }
                depth == 0
            }
            None => false,
        },
        None => false,
    }
}

/// Checks if a block contains an indexer expression
fn check_for_indexer(src: &str) -> Result<bool> {
    let mut exp = Expression::from(src)?;
    let mut depth = 1;
    while let Some(expr) = &exp {
        match expr.expression_type {
            ExpressionType::Comment | ExpressionType::Escaped => continue,
            ExpressionType::Open => {
                if contains_indexer(expr.content, depth - 1) {
                    return Ok(true);
                } else {
                    depth += 1;
                }
            }
            ExpressionType::Close => {
                depth -= 1;
                if depth == 0 {
                    return Ok(false);
                }
            }
            _ => {
                if contains_indexer(expr.content, depth - 1) {
                    return Ok(true);
                }
            }
        }
        exp = expr.next()?;
    }
    Ok(false)
}

/// Checks if a block contains an else block
fn check_for_else(src: &str) -> Result<bool> {
    let mut exp = Expression::from(src)?;
    let mut depth = 1;
    while let Some(expr) = &exp {
        match expr.expression_type {
            ExpressionType::Comment | ExpressionType::Escaped => continue,
            ExpressionType::Open => depth += 1,
            ExpressionType::Close => {
                depth -= 1;
                if depth == 0 {
                    return Ok(false);
                }
            }
            _ => {
                if expr.content == "else" && depth == 1 {
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
        let indexer = check_for_indexer(expression.postfix).map(|found| match found {
            true => {
                let indexer = format!("i_{}", compile.open_stack.len());
                rust.code.push_str("let mut ");
                rust.code.push_str(indexer.as_str());
                rust.code.push_str(" = 0;");
                Some(indexer)
            }
            false => None,
        })?;
        let local = read_local(&next, expression)?;
        let has_else = check_for_else(expression.postfix)?;
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
            indexer,
            has_else,
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
        if let Some(indexer) = &self.indexer {
            rust.code.push_str(indexer);
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
        Ok(match name {
            "index" => rust.code.push_str(self.indexer.as_ref().unwrap()),
            "key" => self.write_map_var(depth, ".0", rust),
            "value" => self.write_map_var(depth, ".1", rust),
            _ => Err(ParseError::new(
                &format!("unexpected variable {}", name),
                expression,
            ))?,
        })
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
