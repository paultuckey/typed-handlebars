mod codegen;
mod parser;

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

/// Where a template came from, for error messages.
struct Source<'a> {
    /// Path to the `.hbs` file, when the template is a file.
    path: Option<&'a str>,
}

/// Renders a template error the way a Handlebars author needs to read it: their file, their line,
/// and nothing about Rust.
fn describe(error: &ParseError, content: &str, source: &Source<'_>) -> String {
    let mut position = String::new();
    if let Some(offset) = error.offset_in(content) {
        let before = &content[..offset];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(offset, str::len) + 1;
        position = match source.path {
            Some(path) => format!("{}:{}:{}: ", path, line, column),
            None => format!("line {}, column {}: ", line, column),
        };
    } else if let Some(path) = source.path {
        position = format!("{}: ", path);
    }
    format!("{}{}", position, error)
}

fn generate_code_for_content(
    name: &str,
    content: &str,
    path_for_include: Option<&str>,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), String> {
    let struct_name_str = name.replace("-", "_");
    let struct_name = format_ident!("{}", struct_name_str);

    let mut block_map = HashMap::new();
    add_builtins(&mut block_map);

    let source = Source {
        path: path_for_include,
    };

    // The template states its own contract; read it before generating anything.
    let context = context::build(content).map_err(|e| describe(&e, content, &source))?;
    let types = codegen::generate(&struct_name_str, &context);

    let options = Options {
        root_var_name: Some("self"),
        write_var_name: "f",
    };
    let compiler = Compiler::new(options, block_map);
    let rust_code = compiler
        .compile(content)
        .map_err(|e| describe(&e, content, &source))?;
    let render_body: proc_macro2::TokenStream = rust_code.code.parse().map_err(|_| {
        // Reaching here means the parser accepted something code generation could not express.
        // That is a bug in this crate rather than in the template, so say so.
        format!(
            "{}internal error: dry-handlebars generated invalid Rust for this template. \
             Please report it at https://github.com/paultuckey/dry-handlebars/issues",
            source
                .path
                .map(|path| format!("{}: ", path))
                .unwrap_or_default()
        )
    })?;

    let method_name = format_ident!("{}", to_snake_case(&struct_name_str));

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

    let function_def = quote! {
        pub fn #method_name<#(#params),*>(#(#names: #field_types),*) -> #struct_name<#(#params),*>
        #where_clause
        {
            #struct_name::new(#(#names),*)
        }
    };

    let include_bytes_stmt = if let Some(path_str) = path_for_include {
        quote! {
            // ensure the compiler is aware the output is linked to the source so that any changes
            // to the hbs file will trigger a recompilation
            const _: &[u8] = include_bytes!(#path_str);
        }
    } else {
        quote! {}
    };

    let struct_def = quote! {
        #include_bytes_stmt

        #(#nested)*

        #builder

        pub struct #struct_name<#(#params),*> #where_clause {
            #(#declarations),*
        }

        impl<#(#params),*> #struct_name<#(#params),*> #where_clause {
            pub fn new(#(#names: #field_types),*) -> Self {
                Self {
                    #(#initialisers),*
                }
            }

            pub fn render(&self) -> String {
                use std::fmt::Write;
                let mut f = String::new();
                let mut render_inner = || -> std::fmt::Result {
                    #render_body
                    Ok(())
                };
                render_inner().unwrap();
                f
            }
        }
    };

    Ok((struct_def, function_def))
}

fn generate_code_for_file(
    path: &Path,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), String> {
    let path_str = path.to_string_lossy();
    let file_stem = match path.file_stem() {
        Some(stem) => stem.to_string_lossy(),
        None => return Err(format!("{}: not a usable template file name", path_str)),
    };
    let content =
        fs::read_to_string(path).map_err(|e| format!("{}: could not be read: {}", path_str, e))?;
    generate_code_for_content(&file_stem, &content, Some(&path_str))
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

#[proc_macro]
pub fn dry_handlebars_directory(input: TokenStream) -> TokenStream {
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

    let mut structs = Vec::new();
    let mut functions = Vec::new();
    let mut errors = Vec::new();

    for entry in WalkDir::new(&root_path).sort_by_file_name() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "hbs") {
            // One broken template reports itself and the rest still compile, so a single typo
            // doesn't bury the whole directory in errors.
            match generate_code_for_file(path) {
                Ok((struct_def, function_def)) => {
                    structs.push(struct_def);
                    functions.push(function_def);
                }
                Err(message) => {
                    errors.push(syn::Error::new(dir_lit.span(), message).to_compile_error())
                }
            }
        }
    }

    let expanded = quote! {
        #(#errors)*
        #(#structs)*
        #(#functions)*
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn dry_handlebars_file(input: TokenStream) -> TokenStream {
    let file_lit = parse_macro_input!(input as LitStr);
    let file_str = file_lit.value();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let path = Path::new(&manifest_dir).join(&file_str);

    if !path.exists() {
        return syn::Error::new(file_lit.span(), format!("File not found: {:?}", path))
            .to_compile_error()
            .into();
    }

    let (struct_def, function_def) = match generate_code_for_file(&path) {
        Ok(generated) => generated,
        Err(message) => {
            return syn::Error::new(file_lit.span(), message)
                .to_compile_error()
                .into();
        }
    };

    let expanded = quote! {
        #struct_def
        #function_def
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn dry_handlebars_str(input: TokenStream) -> TokenStream {
    let StrInput { name, content } = parse_macro_input!(input as StrInput);
    let (struct_def, function_def) =
        match generate_code_for_content(&name.value(), &content.value(), None) {
            Ok(generated) => generated,
            Err(message) => {
                return syn::Error::new(content.span(), message)
                    .to_compile_error()
                    .into();
            }
        };

    let expanded = quote! {
        #struct_def
        #function_def
    };

    TokenStream::from(expanded)
}
