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

//! Handlebars template parser and compiler
//!
//! This crate provides the core functionality for parsing and compiling Handlebars templates
//! into Rust code. It's used internally by the `rusty-handlebars` crate to process templates
//! at compile time.
//!
//! # Features
//!
//! - Handlebars template parsing
//! - Template compilation to Rust code
//! - Support for all standard Handlebars features:
//!   - Variables and expressions
//!   - Block helpers (if, unless, each, with)
//!   - Partials
//!   - Comments
//!   - HTML escaping
//!   - Whitespace control
//!   - Subexpressions
//!   - Lookup helpers
//!
//! # Example
//!
//! ```ignore
//! use compiler::{Compiler, Options, BlockMap};
//! use block::add_builtins;
//!
//! let mut factories = BlockMap::new();
//! add_builtins(&mut factories);
//!
//! let compiler = Compiler::new(Options {
//!     write_var_name: "f",
//!     root_var_name: Some("self")
//! }, factories);
//!
//! let template = "Hello {{name}}!";
//! let rust_code = compiler.compile(template).unwrap();
//! ```
//!
//! # Module Structure
//!
//! - `compiler.rs`: Main compiler implementation
//! - `block.rs`: Block helper implementations
//! - `expression.rs`: Expression parsing and evaluation
//! - `expression_tokenizer.rs`: Tokenization of expressions
//! - `error.rs`: Error types and handling
//! - `build_helper.rs`: Helper functions for template building
