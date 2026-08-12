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

//! Handlebars template parser and compiler.
//!
//! Parses a Handlebars template and compiles it to Rust source. This is the parsing half of
//! `typed-handlebars-macros`, derived from the parser in
//! [rusty-handlebars](https://github.com/h-i-v-e/rusty-handlebars) — see the MIT notice above and
//! the `NOTICE` file at the repository root. The type inference and code generation built on top
//! of it ([`context`](super::context), `codegen`, `assemble`) are not from there.
//!
//! # What is parsed
//!
//! - Variables and paths — `{{ name }}`, `{{ person.name }}`, `{{ ../name }}`
//! - Block helpers — `{{#if}}`, `{{#unless}}`, `{{#each}}`, `{{#with}}`, with `{{else}}` and
//!   block params (`as |row|`)
//! - Comments, whitespace control (`{{~ … ~}}`), raw blocks, and escaped `\{{`
//! - HTML escaping: `{{ }}` escapes, `{{{ }}}` does not
//!
//! Partials are handled before this point, by splicing in `assemble.rs`, so `{{> row}}` never
//! reaches the compiler as a partial.
//!
//! Custom helpers, subexpressions and `{{lookup}}` are **rejected by name** rather than parsed —
//! a helper is Rust code, and a template that needs Rust code stops being something a designer can
//! own. See the supported-subset tables in the README.
//!
//! # Example
//!
//! ```ignore
//! let mut factories = BlockMap::new();
//! add_builtins(&mut factories);
//!
//! let compiler = Compiler::new(
//!     Options {
//!         root_var_name: Some("self"),
//!         write_var_name: "f",
//!         runtime: "::typed_handlebars".to_string(),
//!     },
//!     factories,
//! );
//!
//! let rust_code = compiler.compile("Hello {{name}}!")?;
//! ```
//!
//! # Module structure
//!
//! - `compiler.rs`: main compiler implementation
//! - `block.rs`: block helper implementations
//! - `context.rs`: infers the context shape a template implies (not from rusty-handlebars)
//! - `expression.rs`: expression parsing and evaluation
//! - `expression_tokenizer.rs`: tokenization of expressions
//! - `error.rs`: error types, and the positions they report against the `.hbs` file
