//! The proc-macro half of [`typed-handlebars`](https://docs.rs/typed-handlebars).
//!
//! This crate is an implementation detail: it is a `proc-macro` crate, so it cannot re-export the
//! runtime items the code it generates needs. Depend on `typed-handlebars` instead, which
//! re-exports these three macros under shorter names and documents them with worked examples:
//!
//! | Here                            | What you use            |
//! |---------------------------------|-------------------------|
//! | [`typed_handlebars_directory!`] | `typed_handlebars::directory!` |
//! | [`typed_handlebars_file!`]      | `typed_handlebars::file!`      |
//! | [`typed_handlebars_str!`]       | `typed_handlebars::str!`       |
//!
//! Generated code refers to the runtime crate by whatever name the consumer gave it, so renaming
//! the dependency is fine.

// The parser and code generator are ordinary safe Rust; the inherited parser carried no unsafe
// either, and this keeps it that way.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod assemble;
mod codegen;
mod parser;

use crate::assemble::Assembly;
use crate::parser::block::add_builtins;
use crate::parser::compiler::{Compiler, Options};
use crate::parser::context;
use crate::parser::error::ParseError;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use syn::{LitStr, Token, parse::Parse, parse::ParseStream, parse_macro_input};
use walkdir::WalkDir;

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            for lc in c.to_lowercase() {
                result.push(lc);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// The path generated code should use to reach the runtime crate.
///
/// A consumer may rename the dependency (`hb = { package = "typed-handlebars" }`), in which case
/// `::typed_handlebars` does not resolve for them. Asking Cargo what they called it keeps generated
/// code working without them having to know it needed a particular name.
fn runtime_crate() -> proc_macro2::TokenStream {
    match proc_macro_crate::crate_name("typed-handlebars") {
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let ident = format_ident!("{}", name);
            quote! { ::#ident }
        }
        // Inside `typed-handlebars` itself, which is how its own tests expand.
        //
        // Not `crate`: a doctest is compiled as its own crate that *depends* on
        // `typed-handlebars`, yet `CARGO_MANIFEST_DIR` still points at this manifest, so
        // `crate_name` answers `Itself` for it too. `crate::Truthy` would then look for the trait
        // in the doctest. `extern crate self as typed_handlebars` in the runtime crate makes the
        // absolute path resolve from inside and outside alike.
        Ok(proc_macro_crate::FoundCrate::Itself) => quote! { ::typed_handlebars },
        // Not in the dependency list at all. Emitting the canonical name gives the developer a
        // "use of undeclared crate" pointing at the right name to add.
        Err(_) => quote! { ::typed_handlebars },
    }
}

/// Rust's keywords, which a Handlebars author has no reason to avoid.
///
/// Includes the reserved ones, so a template does not start failing when a future edition takes a
/// word into use.
const KEYWORDS: &[&str] = &[
    "as", "async", "await", "become", "box", "break", "const", "continue", "crate", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Turns a name from a template or a filename into a usable Rust identifier.
///
/// The person writing `.hbs` files picks these names and has no reason to know what Rust reserves,
/// so a variable called `type` or a file called `mod.hbs` has to work. Raw identifiers would cover
/// most of it, but `r#self` and `r#crate` are not legal, so a trailing underscore is used
/// throughout instead — one rule rather than two, and the setter shows up in autocomplete either
/// way.
pub(crate) fn sanitise_ident(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 1);
    for (index, character) in name.chars().enumerate() {
        if index == 0 && character.is_numeric() {
            // An identifier cannot start with a digit, but `2col.hbs` is a reasonable file name.
            out.push('_');
        }
        if character.is_alphanumeric() || character == '_' {
            out.push(character);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    if KEYWORDS.contains(&out.as_str()) {
        out.push('_');
    }
    out
}

/// Shortens a path for display, relative to the crate being built.
///
/// Absolute paths to a build directory are noise in an error message; `templates/results.hbs` is
/// what the developer recognises.
pub(crate) fn relative(path: &str) -> &str {
    let Ok(root) = std::env::var("CARGO_MANIFEST_DIR") else {
        return path;
    };
    match path.strip_prefix(root.as_str()) {
        Some(rest) => match rest.trim_start_matches(['/', '\\']) {
            "" => path,
            trimmed => trimmed,
        },
        None => path,
    }
}

/// Renders a template error the way a Handlebars author needs to read it: their file, their line,
/// and nothing about Rust.
///
/// The position is mapped back through the assembly, so an error inside a partial names the
/// partial's own file rather than wherever it happened to be spliced.
fn describe(error: &ParseError, assembly: &Assembly) -> String {
    let position = match error
        .offset_in(&assembly.text)
        .and_then(|o| assembly.locate(o))
    {
        Some(location) => match &location.path {
            Some(path) => format!("{}:{}:{}: ", relative(path), location.line, location.column),
            None => format!("line {}, column {}: ", location.line, location.column),
        },
        None => match assembly.path() {
            Some(path) => format!("{}: ", relative(path)),
            None => String::new(),
        },
    };
    format!("{}{}", position, error)
}

fn generate_code_for_content(
    name: &str,
    assembly: &Assembly,
) -> Result<proc_macro2::TokenStream, String> {
    let content = &assembly.text;
    let template_name = sanitise_ident(name);
    // Everything a template generates lives in a module of its own, so a template with a `rows`
    // list cannot collide with a template called `rows_item`, and the names stay readable.
    let module_name = format_ident!("{}", template_name);
    let struct_name = format_ident!("Template");

    let mut block_map = HashMap::new();
    add_builtins(&mut block_map);

    // The template states its own contract; read it before generating anything.
    let context = context::build(content).map_err(|e| describe(&e, assembly))?;
    let runtime = runtime_crate();
    let types = codegen::generate(&context, &runtime);

    let options = Options {
        root_var_name: Some("self"),
        write_var_name: "f",
        runtime: runtime.to_string(),
    };
    let compiler = Compiler::new(options, block_map);
    let rust_code = compiler
        .compile(content)
        .map_err(|e| describe(&e, assembly))?;
    let render_body: proc_macro2::TokenStream = rust_code.code.parse().map_err(|_| {
        // Reaching here means the parser accepted something code generation could not express.
        // That is a bug in this crate rather than in the template, so say so.
        format!(
            "{}internal error: typed-handlebars generated invalid Rust for this template. \
             Please report it at https://github.com/paultuckey/typed-handlebars/issues",
            assembly
                .path()
                .map(|path| format!("{}: ", relative(path)))
                .unwrap_or_default()
        )
    })?;

    let method_name = format_ident!("{}", to_snake_case(&template_name));

    let codegen::Types {
        nested,
        builder,
        params,
        predicates,
        declarations,
        initialisers,
        names,
        types: field_types,
    } = types;

    // A list's bound has to be on the declaration as well as the impl, because the field type is
    // the container rather than the item.
    let where_clause = codegen::where_clause(&predicates);

    // Generated items carry documentation so that a consumer denying `missing_docs` needs no
    // `#[allow]`, and so IDE autocomplete says what each setter is for.
    let module_doc = format!("Types generated from the `{}` template.", template_name);
    let struct_doc = format!("The `{}` template.", template_name);
    let new_doc =
        "Creates it from every variable the template uses, in the order it first mentions them.";
    let fn_doc = format!(
        "Builds the `{}` template. See [`{}::Builder`] to name the variables instead.",
        template_name, template_name
    );

    // The function is defined inside the module, where the generated types are in scope, then
    // re-exported beside it. `templates::page(…)` and `templates::page::RowsItem` both work,
    // because a module and a function do not share a namespace.
    let function_def = quote! {
        #[doc = #fn_doc]
        pub fn #method_name<#(#params),*>(
            #(#names: #field_types),*
        ) -> #struct_name<#(#params),*>
        #where_clause
        {
            #struct_name::new(#(#names),*)
        }
    };

    // Every file that went into this template, partials included, so editing any of them triggers
    // a rebuild of the code generated from it.
    let includes = &assembly.includes;
    let include_bytes_stmt = quote! {
        #(const _: &[u8] = ::core::include_bytes!(#includes);)*
    };

    let struct_def = quote! {
        #[doc = #module_doc]
        pub mod #module_name {
        #include_bytes_stmt

        #(#nested)*

        #builder

        #[doc = #struct_doc]
        pub struct #struct_name<#(#params),*> #where_clause {
            #(#declarations),*
        }

        impl<#(#params),*> #struct_name<#(#params),*> #where_clause {
            #[doc = #new_doc]
            pub fn new(#(#names: #field_types),*) -> Self {
                Self {
                    #(#initialisers),*
                }
            }

            /// Writes the template into any sink — a `String`, a response buffer, anything
            /// implementing `fmt::Write`.
            ///
            /// This is where the rendering actually happens; `render` is a convenience over it.
            pub fn render_to(
                &self,
                f: &mut impl ::core::fmt::Write,
            ) -> ::core::fmt::Result {
                // Written values go through `Render`, which generated code reaches by method
                // syntax so that method lookup can step through the references a loop body holds.
                // Anonymous, so it can never collide with a name from the template.
                #[allow(unused_imports)]
                use #runtime::{Length as _, RenderExt as _};
                // A template with no variables writes nothing, so the sink can go unused.
                let _ = &f;
                #render_body
                ::core::result::Result::Ok(())
            }

            /// Renders the template to a new `String`.
            pub fn render(&self) -> ::std::string::String {
                let mut out = ::std::string::String::new();
                // Writing into a `String` cannot fail, so there is no error to propagate.
                let _ = self.render_to(&mut out);
                out
            }
        }

        // Rendering into a formatter means a nested template can be passed straight to a parent as
        // a value and written once into its buffer, rather than each level allocating a `String`.
        impl<#(#params),*> ::core::fmt::Display for #struct_name<#(#params),*> #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                self.render_to(f)
            }
        }

        #function_def
        }

        #[doc(inline)]
        pub use #module_name::#method_name;
    };

    Ok(struct_def)
}

fn generate_code_for_file(
    path: &Path,
    partials: &Path,
) -> Result<proc_macro2::TokenStream, String> {
    let path_str = path.to_string_lossy();
    let file_stem = match path.file_stem() {
        Some(stem) => stem.to_string_lossy(),
        None => return Err(format!("{}: not a usable template file name", path_str)),
    };
    let content = fs::read_to_string(path)
        .map_err(|e| format!("{}: could not be read: {}", relative(&path_str), e))?;
    let assembly = Assembly::build(&content, Some(&path_str), Some(partials))
        .map_err(|e| format!("{}: {}", relative(&path_str), e))?;
    generate_code_for_content(&file_stem, &assembly)
}

struct StrInput {
    name: LitStr,
    content: LitStr,
}

impl Parse for StrInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let content: LitStr = input.parse()?;
        // A trailing comma is allowed, so the template can sit on its own line.
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(StrInput { name, content })
    }
}

/// Templates grouped the way the directory groups them.
#[derive(Default)]
struct Tree {
    templates: Vec<proc_macro2::TokenStream>,
    children: std::collections::BTreeMap<String, Tree>,
}

impl Tree {
    fn insert(&mut self, branch: &[String], generated: proc_macro2::TokenStream) {
        match branch.split_first() {
            None => self.templates.push(generated),
            Some((head, rest)) => self
                .children
                .entry(head.clone())
                .or_default()
                .insert(rest, generated),
        }
    }

    fn emit(&self) -> proc_macro2::TokenStream {
        let templates = &self.templates;
        let children = self.children.iter().map(|(name, child)| {
            let ident = format_ident!("{}", crate::sanitise_ident(name));
            let doc = format!("Templates from the `{}` directory.", name);
            let inner = child.emit();
            quote! {
                #[doc = #doc]
                pub mod #ident {
                    #inner
                }
            }
        });
        quote! {
            #(#templates)*
            #(#children)*
        }
    }
}

// Documented on the re-export in the runtime crate, where the examples can actually run. Docs
// here would be concatenated onto those by `#[doc(inline)]`, so they are deliberately absent.
#[allow(missing_docs)]
#[proc_macro]
pub fn typed_handlebars_directory(input: TokenStream) -> TokenStream {
    let dir_lit = parse_macro_input!(input as LitStr);
    let dir_str = dir_lit.value();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let root_path = Path::new(&manifest_dir).join(&dir_str);

    if !root_path.exists() {
        return syn::Error::new(
            dir_lit.span(),
            format!("Directory not found: {:?}", root_path),
        )
        .to_compile_error()
        .into();
    }

    let mut tree = Tree::default();
    let mut errors = Vec::new();

    for entry in WalkDir::new(&root_path).sort_by_file_name() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|ext| ext != "hbs") {
            continue;
        }

        // The directory layout is something the template author expressed, so mirror it as modules
        // rather than flattening it — `templates/admin/row.hbs` becomes `templates::admin::row`.
        let branch = match path
            .parent()
            .and_then(|dir| dir.strip_prefix(&root_path).ok())
        {
            Some(relative) => relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect(),
            None => Vec::new(),
        };

        // One broken template reports itself and the rest still compile, so a single typo
        // doesn't bury the whole directory in errors.
        match generate_code_for_file(path, &root_path) {
            Ok(generated) => tree.insert(&branch, generated),
            Err(message) => {
                errors.push(syn::Error::new(dir_lit.span(), message).to_compile_error())
            }
        }
    }

    let templates = tree.emit();
    let expanded = quote! {
        #(#errors)*
        #templates
    };

    TokenStream::from(expanded)
}

// Documented on the re-export in the runtime crate — see `typed_handlebars_directory` above.
#[allow(missing_docs)]
#[proc_macro]
pub fn typed_handlebars_file(input: TokenStream) -> TokenStream {
    let file_lit = parse_macro_input!(input as LitStr);
    let file_str = file_lit.value();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let path = Path::new(&manifest_dir).join(&file_str);

    if !path.exists() {
        return syn::Error::new(file_lit.span(), format!("File not found: {:?}", path))
            .to_compile_error()
            .into();
    }

    let partials = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let expanded = match generate_code_for_file(&path, &partials) {
        Ok(generated) => generated,
        Err(message) => {
            return syn::Error::new(file_lit.span(), message)
                .to_compile_error()
                .into();
        }
    };

    TokenStream::from(expanded)
}

// Documented on the re-export in the runtime crate — see `typed_handlebars_directory` above.
#[allow(missing_docs)]
#[proc_macro]
pub fn typed_handlebars_str(input: TokenStream) -> TokenStream {
    let StrInput { name, content } = parse_macro_input!(input as StrInput);
    // A `str!` template has no directory, so it has nowhere to resolve partials from.
    let assembled = Assembly::build(&content.value(), None, None)
        .and_then(|assembly| generate_code_for_content(&name.value(), &assembly));
    let expanded = match assembled {
        Ok(generated) => generated,
        Err(message) => {
            return syn::Error::new(content.span(), message)
                .to_compile_error()
                .into();
        }
    };

    TokenStream::from(expanded)
}
