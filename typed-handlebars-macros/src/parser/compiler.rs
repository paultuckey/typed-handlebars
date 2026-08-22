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

//! Handlebars template compilation
//!
//! This module provides functionality for compiling Handlebars templates into Rust code.
//! It handles:
//! - Variable resolution and scope management
//! - Block helper compilation
//! - Expression evaluation
//! - HTML escaping
//!
//! # Compilation Process
//!
//! The compilation process involves:
//! 1. Parsing the template into expressions
//! 2. Resolving variables and scopes
//! 3. Compiling block helpers
//! 4. Generating Rust code
//!
//! # Examples
//!
//! Basic usage:
//! ```ignore
//! use compiler::{Compiler, Options};
//! use block::add_builtins;
//!
//! let mut block_map = HashMap::new();
//! add_builtins(&mut block_map);
//!
//! let options = Options {
//!     root_var_name: Some("data"),
//!     write_var_name: "write"
//! };
//!
//! let compiler = Compiler::new(options, block_map);
//! let rust = compiler.compile("Hello {{name}}!")?;
//! ```
//!
//! Complex template example:
//! ```ignore
//! use compiler::{Compiler, Options};
//! use block::add_builtins;
//!
//! let mut block_map = HashMap::new();
//! add_builtins(&mut block_map);
//!
//! let options = Options {
//!     root_var_name: Some("data"),
//!     write_var_name: "write"
//! };
//!
//! let template = r#"
//! <div class="user-profile">
//!     {{#if user}}
//!         <h1>{{user.name}}</h1>
//!         {{#if user.bio}}
//!             <p class="bio">{{user.bio}}</p>
//!         {{else}}
//!             <p class="no-bio">No bio available</p>
//!         {{/if}}
//!         
//!         {{#if_some user.posts as post}}
//!             <div class="posts">
//!                 <h2>Posts</h2>
//!                 {{#each post as post}}
//!                     <article class="post">
//!                         <h3>{{post.title}}</h3>
//!                         <p>{{post.content}}</p>
//!                         <div class="meta">
//!                             <span>Posted on {{post.date}}</span>
//!                             {{#if post.tags}}
//!                                 <div class="tags">
//!                                     {{#each post.tags as tag}}
//!                                         <span class="tag">{{tag}}</span>
//!                                     {{/each}}
//!                                 </div>
//!                             {{/if}}
//!                         </div>
//!                     </article>
//!                 {{/each}}
//!             </div>
//!         {{/if_some}}
//!     {{else}}
//!         <p>Please log in to view your profile</p>
//!     {{/if}}
//! </div>
//! "#;
//!
//! let compiler = Compiler::new(options, block_map);
//! let rust = compiler.compile(template)?;
//! ```
//!
//! This example demonstrates:
//! - Nested conditional blocks with `if` and `else`
//! - Option handling with `if_some`
//! - Collection iteration with `each`
//! - HTML escaping for safe output
//! - Complex variable resolution
//! - Block scope management
//! - Template structure and formatting

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use regex::{Captures, Regex};

use crate::parser::{
    else_branch::{self, ElseBranch},
    error::{ParseError, Result},
    expression::{Expression, ExpressionType},
    expression_tokenizer::{Token, TokenType},
    path,
};

/// Local variable declaration in a block
pub enum Local {
    /// Named local variable: `as name`
    As(String),
    /// This context: `this`
    This,
    /// No local variable
    None,
}

/// A scope in the template
pub struct Scope {
    /// The block that opened this scope
    pub opened: Box<dyn Block>,
    /// The depth of this scope
    pub depth: usize,
}

/// A pending write operation
enum PendingWrite<'a> {
    /// Raw text to write
    Raw(&'a str),
    /// Expression to evaluate and write
    Expression(Expression<'a>, Escaping),
}

/// Rust code generation state
pub struct Rust {
    /// Generated code
    pub code: String,
    /// Top level variables
    pub top_level_vars: HashSet<String>,
    /// Whether the template called a helper, and so needs the frame passed to `render`.
    pub uses_frame: bool,
}

/// Whether a written value goes through the runtime's HTML escaper.
///
/// This is the difference Handlebars promises and this crate used not to deliver — both forms once
/// emitted identical code, so `{{ }}` silently passed markup through.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Escaping {
    /// `{{{ raw }}}` — straight out, exactly as given.
    None,
    /// `{{ escaped }}` — through `escape`.
    Html,
}

impl Rust {
    /// Creates a new Rust code generator
    pub fn new() -> Self {
        Self {
            code: String::new(),
            top_level_vars: HashSet::new(),
            uses_frame: false,
        }
    }
}

/// Trait for block helpers
pub trait Block {
    /// Handles block closing
    fn handle_close(&self, rust: &mut Rust) {
        rust.code.push('}');
    }

    /// Resolves a private variable
    fn resolve_private<'a>(
        &self,
        _depth: usize,
        expression: &'a Expression<'a>,
        _name: &str,
        _rust: &mut Rust,
    ) -> Result<()> {
        Err(ParseError::new(
            &format!("{} not expected ", expression.content),
            expression,
        ))
    }

    /// Handles else block
    fn handle_else<'a>(&self, expression: &'a Expression<'a>, _rust: &mut Rust) -> Result<()> {
        Err(ParseError::new("else not expected here", expression))
    }

    /// Whether this block supplies `@…` variables to its body.
    ///
    /// Only `{{#each}}` does. `{{#if}}`, `{{#unless}}` and `{{#with}}` are **transparent** to an
    /// `@…` lookup, as they are in handlebars.js, so a reference inside one belongs to the
    /// enclosing loop rather than failing. See [`Compile::find_private_scope`].
    fn supplies_privates(&self) -> bool {
        false
    }

    /// Whether an `{{else if}}` can chain onto this block.
    ///
    /// Only `{{#if}}` and `{{#unless}}`. The chain compiles to a Rust `else if`, so it needs this
    /// block's own alternative branch to be a plain `else` and its close to be a single `}` —
    /// which is what makes a chain of any length still close with one brace. `{{#each}}`'s
    /// `{{else}}` is an emptiness test wrapped in its own scope, so chaining onto it would have to
    /// nest instead, and `{{#with}}` has no alternative branch at all.
    fn allows_else_if(&self) -> bool {
        false
    }

    /// Returns the this context
    fn this(&self) -> Option<&str> {
        None
    }

    /// Returns the local variable
    fn local(&self) -> &Local {
        &Local::None
    }
}

/// Trait for block helper factories
pub trait BlockFactory {
    /// Opens a new block
    fn open<'a>(
        &self,
        compile: &'a Compile<'a>,
        token: Token<'a>,
        expression: &'a Expression<'a>,
        rust: &mut Rust,
    ) -> Result<Box<dyn Block>>;
}

/// Map of block helper names to factories
pub type BlockMap = HashMap<&'static str, &'static dyn BlockFactory>;

/// Compiler state
pub struct Compile<'a> {
    /// Stack of open blocks
    pub open_stack: Vec<Scope>,
    /// Map of block helpers
    pub block_map: &'a BlockMap,
    /// How generated code reaches the runtime crate.
    pub runtime: &'a str,
}

/// The local the generated `render` binds the frame to.
///
/// It cannot collide with anything from a template: a template's own variables are reached as
/// `self.…` or through a depth-suffixed loop local, never as a bare name.
pub const FRAME_VAR: &str = "cx";

/// Writes `text` as a Rust string literal.
///
/// Handlebars escapes the quote that delimits a literal, so `\"` inside `"…"` is the quote itself
/// rather than two characters. The escape is unwound here and rewritten for Rust, which spells
/// some of the same characters differently.
fn push_string_literal(buffer: &mut String, text: &str) {
    buffer.push('"');
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        match character {
            '\\' => match chars.next() {
                Some(escaped @ ('"' | '\'' | '\\')) => push_escaped(buffer, escaped),
                Some(other) => {
                    push_escaped(buffer, '\\');
                    push_escaped(buffer, other);
                }
                None => push_escaped(buffer, '\\'),
            },
            character => push_escaped(buffer, character),
        }
    }
    buffer.push('"');
}

fn push_escaped(buffer: &mut String, character: char) {
    match character {
        '"' => buffer.push_str("\\\""),
        '\\' => buffer.push_str("\\\\"),
        '\n' => buffer.push_str("\\n"),
        '\r' => buffer.push_str("\\r"),
        '\t' => buffer.push_str("\\t"),
        character => buffer.push(character),
    }
}

/// Writes a dotted template path as Rust field access, one sanitised segment at a time.
///
/// A template may name a field `type` or `match`; the generated code cannot.
fn push_path(buffer: &mut String, path: &str, leading_dot: bool) {
    for (index, segment) in path.split('.').enumerate() {
        if index > 0 || leading_dot {
            buffer.push('.');
        }
        buffer.push_str(&crate::sanitise_ident(segment));
    }
}

/// Appends a depth suffix to a variable name
pub fn append_with_depth(depth: usize, var: &str, buffer: &mut String) {
    buffer.push_str(&crate::sanitise_ident(var));
    buffer.push('_');
    buffer.push_str(depth.to_string().as_str());
}

/// Root block implementation
struct Root<'a> {
    this: Option<&'a str>,
}

impl<'a> Block for Root<'a> {
    fn this<'b>(&self) -> Option<&str> {
        self.this
    }
}

impl<'a> Compile<'a> {
    /// Creates a new compiler
    fn new(this: Option<&'static str>, block_map: &'a BlockMap, runtime: &'a str) -> Self {
        Self {
            open_stack: vec![Scope {
                depth: 0,
                opened: Box::new(Root { this }),
            }],
            block_map,
            runtime,
        }
    }

    /// Finds the scope for a variable
    fn find_scope(&self, var: &'a str) -> Result<(&'a str, &Scope)> {
        let mut scope = self.open_stack.last().unwrap();
        let mut local = var;
        while local.starts_with("../") {
            match scope.depth {
                0 => {
                    return Err(ParseError::general(&format!(
                        "`{}` reaches above the top of the template",
                        var
                    )));
                }
                _ => {
                    local = &local[3..];
                    scope = self.open_stack.get(scope.depth - 1).unwrap();
                }
            }
        }
        Ok((local, scope))
    }

    /// Finds the block that supplies an `@…` variable.
    ///
    /// Deliberately not [`Self::find_scope`]. A private lives on a loop, not on a scope, so two
    /// things differ — both checked against handlebars.js rather than inferred:
    ///
    /// - **Blocks that supply nothing are transparent.** `{{#each xs}}{{#if a}}{{@index}}{{/if}}`
    ///   reads the loop's index; the `{{#if}}` is not in the way.
    /// - **`../` steps out one loop, not one scope.** With an intervening `{{#if}}` *or*
    ///   `{{#with}}`, `{{@../index}}` still lands on the enclosing `{{#each}}`.
    ///
    /// [`super::block::check_for_privates`] counts a body's nesting the same way, because the two
    /// have to agree about which loop a reference belongs to.
    fn find_private_scope(&self, var: &'a str) -> Result<(&'a str, &Scope)> {
        let mut local = var;
        let mut index = self.open_stack.len() - 1;
        let mut stepped_out = false;
        loop {
            while !self.open_stack[index].opened.supplies_privates() {
                if index == 0 {
                    return Err(ParseError::general(&format!(
                        "`@{}` {}",
                        var,
                        if stepped_out {
                            "reaches above the outermost `{{#each}}`"
                        } else {
                            "is only available inside an `{{#each}}` block"
                        }
                    )));
                }
                index -= 1;
            }
            match local.strip_prefix("../") {
                Some(rest) => {
                    local = rest;
                    stepped_out = true;
                    // Safe: the loop above only settles on a block that supplies privates, and the
                    // root scope never does, so `index` is at least 1 here.
                    index -= 1;
                }
                None => return Ok((local, &self.open_stack[index])),
            }
        }
    }

    /// Resolves a local variable
    fn resolve_local(
        &self,
        depth: usize,
        var: &'a str,
        local: &'a str,
        buffer: &mut String,
    ) -> bool {
        if var.starts_with(local) {
            let len = local.len();
            if var.len() > len {
                if &var[len..len + 1] != "." {
                    return false;
                }
                append_with_depth(depth, local, buffer);
                push_path(buffer, &var[len + 1..], true);
            } else {
                append_with_depth(depth, local, buffer);
            }
            return true;
        }
        false
    }

    /// Resolves a variable in a scope
    fn resolve_var(&self, var: &'a str, scope: &Scope, rust: &mut Rust) -> Result<()> {
        if scope.depth == 0 {
            if let Some(this) = scope.opened.this() {
                rust.code.push_str(this);
                rust.code.push('.');
            }
            push_path(&mut rust.code, var, false);
            rust.top_level_vars.insert(var.to_string());
            return Ok(());
        }
        if match scope.opened.local() {
            Local::As(local) => self.resolve_local(scope.depth, var, local, &mut rust.code),
            Local::This => {
                rust.code.push_str("this_");
                rust.code.push_str(scope.depth.to_string().as_str());
                if var != "this" {
                    push_path(&mut rust.code, var, true);
                }
                true
            }
            Local::None => false,
        } {
            return Ok(());
        }
        let parent = &self.open_stack[scope.depth - 1];
        if let Some(this) = scope.opened.this() {
            self.resolve_var(this, parent, rust)?;
            if var != this {
                push_path(&mut rust.code, var, true);
            }
        } else {
            self.resolve_var(var, parent, rust)?;
        }
        Ok(())
    }

    /// Resolves a sub-expression
    fn resolve_sub_expression(&self, raw: &str, value: &str, rust: &mut Rust) -> Result<()> {
        self.resolve(
            &Expression {
                expression_type: ExpressionType::Raw,
                prefix: "",
                content: value,
                postfix: "",
                raw,
                standalone: false,
            },
            rust,
        )
    }

    /// Writes a variable expression
    pub fn write_var(
        &self,
        expression: &Expression<'a>,
        rust: &mut Rust,
        var: &Token<'a>,
    ) -> Result<()> {
        match var.token_type {
            // `@root` is absolute — it names the top-level scope from any depth — so it is taken
            // before `find_private_scope`, whose whole job is the outward walk that `@index`,
            // `@first` and `@last` need and this does not.
            TokenType::PrivateVariable if path::under_root(var.value).is_some() => {
                let name = path::under_root(var.value).expect("checked by the guard");
                let root = &self.open_stack[0];
                self.write_place(name, root, rust)?;
            }
            TokenType::PrivateVariable => {
                let (name, scope) = self.find_private_scope(var.value)?;
                scope
                    .opened
                    .resolve_private(scope.depth, expression, name, rust)?;
            }
            TokenType::Variable => {
                let (name, scope) = self.find_scope(var.value)?;
                self.write_place(name, scope, rust)?;
            }
            // Written as-is where the template already spelled Rust — a number — and rewritten
            // where it did not: `{{ 'x' }}` is a string in Handlebars and a char in Rust.
            TokenType::Literal => match var.quoted_text() {
                Some(text) => push_string_literal(&mut rust.code, text),
                None => rust.code.push_str(var.value),
            },
            TokenType::SubExpression(raw) => {
                self.resolve_sub_expression(raw, var.value, rust)?;
            }
        }
        Ok(())
    }

    /// Writes one resolved place, counted if the path ends in `.length`.
    ///
    /// The parenthesis pair is what keeps `.length()` bound to the whole resolved expression,
    /// whatever scope walking turned it into — and the call is method syntax rather than
    /// `Length::length(x)` so that method lookup steps through the reference a loop body holds.
    fn write_place(&self, name: &'a str, scope: &Scope, rust: &mut Rust) -> Result<()> {
        match path::counted(name) {
            Some(subject) => {
                rust.code.push('(');
                self.resolve_var(subject, scope, rust)?;
                rust.code.push_str(").length()");
            }
            None => self.resolve_var(name, scope, rust)?,
        }
        Ok(())
    }

    /// Handles an else block
    fn handle_else(&self, expression: &Expression<'a>, rust: &mut Rust) -> Result<()> {
        match self.open_stack.last() {
            Some(scope) => scope.opened.handle_else(expression, rust),
            None => Err(ParseError::new("else not expected here", expression)),
        }
    }

    /// Handles `{{else}}` and the conditions that chain onto it.
    ///
    /// `{{else if b}}` becomes a Rust `else if`, which is an exact match for what Handlebars means
    /// by it — the chain shares the enclosing block's single closing brace, however long it gets.
    /// The chained helper decides the sense of the test, so `{{else unless b}}` negates whether or
    /// not the block it sits in was an `{{#unless}}`.
    ///
    /// [`context`](super::context) rejects the unchainable forms before this runs, so the errors
    /// here are a backstop rather than the message the template author normally sees.
    fn handle_else_branch(
        &self,
        expression: &Expression<'a>,
        branch: ElseBranch<'a>,
        rust: &mut Rust,
    ) -> Result<()> {
        let keyword = branch.keyword();
        let (negated, condition) = match branch {
            ElseBranch::Plain => return self.handle_else(expression, rust),
            ElseBranch::UnsupportedHelper(helper) => {
                return Err(ParseError::new(
                    &format!("`{{{{else {helper}}}}}` is not supported"),
                    expression,
                ));
            }
            ElseBranch::Chained { negated, condition } => (negated, condition),
        };

        match self.open_stack.last() {
            Some(scope) if scope.opened.allows_else_if() => {}
            Some(_) => {
                return Err(ParseError::new(
                    &format!(
                        "`{{{{{keyword}}}}}` is only supported inside `{{{{#if}}}}` and \
                         `{{{{#unless}}}}`"
                    ),
                    expression,
                ));
            }
            None => return Err(ParseError::new("else not expected here", expression)),
        }

        let var = Token::first(condition)?.ok_or_else(|| {
            ParseError::new(
                &format!("`{{{{{keyword}}}}}` needs something to test"),
                expression,
            )
        })?;
        // Handlebars truthiness, exactly as `{{#if}}` tests it — see `IfOrUnless::new`.
        rust.code.push_str("}else if ");
        if negated {
            rust.code.push('!');
        }
        rust.code.push_str(self.runtime);
        rust.code.push_str("::Truthy::is_truthy(&");
        self.write_var(expression, rust, &var)?;
        rust.code.push_str("){");
        Ok(())
    }

    /// Resolves a helper call — `{{ t "Save" }}` — into a method call on the frame.
    ///
    /// The frame is the value handed to `render` alongside the data, and a helper is one of its
    /// methods. Nothing here checks that the method exists: the generated call does that, and the
    /// compiler's own "no method named `t` found for struct `Ctx`" names both the helper and the
    /// frame better than a guess made here could.
    fn resolve_helper(
        &self,
        expression: &Expression<'a>,
        name: Token<'a>,
        mut args: Token<'a>,
        rust: &mut Rust,
    ) -> Result<()> {
        rust.uses_frame = true;
        rust.code.push_str(FRAME_VAR);
        rust.code.push('.');
        rust.code.push_str(&crate::sanitise_ident(name.value));
        rust.code.push('(');
        self.write_argument(expression, rust, &args)?;
        loop {
            args = match args.next()? {
                Some(token) => {
                    rust.code.push_str(", ");
                    self.write_argument(expression, rust, &token)?;
                    token
                }
                None => {
                    rust.code.push(')');
                    return Ok(());
                }
            };
        }
    }

    /// Writes one helper argument, which always reaches the helper as a `&str`.
    ///
    /// A literal is handed over as the text the template spelled, so `{{ money 123 }}` passes
    /// `"123"`. Anything else is a reference to the caller's data, of whatever type they gave it,
    /// so it goes through the same `Render` path a `{{{ … }}}` would take and arrives as the text
    /// that would have been written.
    fn write_argument(
        &self,
        expression: &Expression<'a>,
        rust: &mut Rust,
        arg: &Token<'a>,
    ) -> Result<()> {
        // A number is a literal here and a path in `{{ 42 }}`, which is the distinction
        // handlebars.js draws: position decides, not spelling.
        match arg.token_type {
            TokenType::Literal => {
                push_string_literal(&mut rust.code, arg.literal_text());
                Ok(())
            }
            TokenType::Variable if arg.numeric() => {
                push_string_literal(&mut rust.code, arg.value);
                Ok(())
            }
            _ => {
                rust.code.push_str("&(");
                self.write_var(expression, rust, arg)?;
                rust.code.push_str(").shown().to_string()");
                Ok(())
            }
        }
    }

    /// Resolves an expression
    fn resolve(&self, expression: &Expression<'a>, rust: &mut Rust) -> Result<()> {
        let token = match Token::first(expression.content)? {
            Some(token) => token,
            None => return Err(ParseError::new("expected token", expression)),
        };
        rust.code.push_str(expression.prefix);
        if let TokenType::SubExpression(raw) = token.token_type {
            self.resolve_sub_expression(raw, token.value, rust)?;
        } else if let Some(args) = token.next()? {
            self.resolve_helper(expression, token, args, rust)?;
        } else {
            self.write_var(expression, rust, &token)?;
        }
        rust.code.push_str(expression.postfix);
        Ok(())
    }

    /// Writes a local variable declaration
    pub fn write_local(&self, rust: &mut String, local: &Local) {
        append_with_depth(
            self.open_stack.len(),
            match local {
                Local::As(local) => local,
                _ => "this",
            },
            rust,
        );
    }

    /// Closes a block
    fn close(&mut self, expression: Expression<'a>, rust: &mut Rust) -> Result<()> {
        let scope = self
            .open_stack
            .pop()
            .ok_or_else(|| ParseError::new("Mismatched block helper", &expression))?;
        scope.opened.handle_close(rust);
        Ok(())
    }

    /// Opens a block
    fn open(&mut self, expression: Expression<'a>, rust: &mut Rust) -> Result<()> {
        let token = Token::first(expression.content)?
            .ok_or_else(|| ParseError::new("expected token", &expression))?;
        match self.block_map.get(token.value) {
            Some(block) => {
                self.open_stack.push(Scope {
                    opened: block.open(self, token, &expression, rust)?,
                    depth: self.open_stack.len(),
                });
                Ok(())
            }
            None => Err(ParseError::new(
                &format!("unsupported block helper {}", token.value),
                &expression,
            )),
        }
    }
}

/// Compiler options
#[derive(Debug, Clone)]
pub struct Options {
    /// Name of the root variable
    pub root_var_name: Option<&'static str>,
    /// Name of the write function
    pub write_var_name: &'static str,
    /// How generated code reaches the runtime crate, which depends on what the consumer called it.
    pub runtime: String,
}

/// Main compiler implementation
pub struct Compiler {
    /// Regex for cleaning whitespace
    clean: Regex,
    /// Compiler options
    options: Options,
    /// Map of block helpers
    block_map: BlockMap,
}

impl Compiler {
    /// Creates a new compiler
    pub fn new(options: Options, block_map: BlockMap) -> Self {
        Self {
            clean: Regex::new("[\\\\\"\\{\\}]").unwrap(),
            options,
            block_map,
        }
    }

    /// Escapes HTML content
    fn escape<'a>(&self, content: &'a str) -> Cow<'a, str> {
        self.clean
            .replace_all(content, |captures: &Captures| match &captures[0] {
                "{" | "}" => format!("{}{}", &captures[0], &captures[0]),
                _ => format!("\\{}", &captures[0]),
            })
    }

    /// Commits pending writes
    fn commit_pending<'a>(
        &self,
        pending: &mut Vec<PendingWrite<'a>>,
        compile: &mut Compile<'a>,
        rust: &mut Rust,
    ) -> Result<()> {
        if pending.is_empty() {
            return Ok(());
        }
        rust.code.push_str("::core::write!(");
        rust.code.push_str(self.options.write_var_name);
        rust.code.push_str(", \"");
        for pending in pending.iter() {
            match pending {
                PendingWrite::Raw(raw) => rust.code.push_str(self.escape(raw).as_ref()),
                PendingWrite::Expression(..) => rust.code.push_str("{}"),
            }
        }
        rust.code.push('"');
        for pending in pending.iter() {
            if let PendingWrite::Expression(expression, escaping) = pending {
                rust.code.push_str(", ");
                // Written through `Render` rather than `Display`, so an `Option` writes nothing
                // instead of failing to compile. Method syntax rather than a call, because a loop
                // body holds an `&Item` where a field is a plain value, and only method lookup
                // steps through that difference on its own. The parentheses keep the method bound
                // to the whole resolved expression.
                rust.code.push('(');
                compile.resolve(
                    &Expression {
                        expression_type: ExpressionType::Raw,
                        prefix: "",
                        content: expression.content,
                        postfix: "",
                        raw: expression.raw,
                        standalone: false,
                    },
                    rust,
                )?;
                rust.code.push_str(match escaping {
                    Escaping::Html => ").escaped()",
                    Escaping::None => ").shown()",
                });
            }
        }
        rust.code.push_str(")?;");
        pending.clear();
        Ok(())
    }

    /// Compiles a template
    pub fn compile(&self, src: &str) -> Result<Rust> {
        let mut compile = Compile::new(
            self.options.root_var_name,
            &self.block_map,
            &self.options.runtime,
        );
        let mut rust = Rust::new();
        let mut pending: Vec<PendingWrite> = Vec::new();
        let mut rest = src;
        let mut expression = Expression::from(src)?;
        while let Some(expr) = expression {
            let Expression {
                expression_type,
                prefix,
                content,
                postfix,
                raw: _,
                standalone: _,
            } = &expr;
            rest = postfix;
            if !prefix.is_empty() {
                pending.push(PendingWrite::Raw(prefix));
            }
            match expression_type {
                ExpressionType::Raw => pending.push(PendingWrite::Expression(expr, Escaping::None)),
                ExpressionType::HtmlEscaped => match else_branch::classify(content) {
                    Some(branch) => {
                        self.commit_pending(&mut pending, &mut compile, &mut rust)?;
                        compile.handle_else_branch(&expr, branch, &mut rust)?
                    }
                    None => pending.push(PendingWrite::Expression(expr, Escaping::Html)),
                },
                ExpressionType::Open => {
                    self.commit_pending(&mut pending, &mut compile, &mut rust)?;
                    compile.open(expr, &mut rust)?
                }
                ExpressionType::Close => {
                    self.commit_pending(&mut pending, &mut compile, &mut rust)?;
                    compile.close(expr, &mut rust)?
                }
                ExpressionType::Escaped => pending.push(PendingWrite::Raw(content)),
                _ => (),
            };
            expression = expr.next()?;
        }
        if !rest.is_empty() {
            pending.push(PendingWrite::Raw(rest));
        }
        self.commit_pending(&mut pending, &mut compile, &mut rust)?;
        Ok(rust)
    }
}
