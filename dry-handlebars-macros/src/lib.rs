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
/// A consumer may rename the dependency (`hb = { package = "dry-handlebars" }`), in which case
/// `::dry_handlebars` does not resolve for them. Asking Cargo what they called it keeps generated
/// code working without them having to know it needed a particular name.
fn runtime_crate() -> proc_macro2::TokenStream {
    match proc_macro_crate::crate_name("dry-handlebars") {
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let ident = format_ident!("{}", name);
            quote! { ::#ident }
        }
        // Inside `dry-handlebars` itself, which is how its own tests expand.
        Ok(proc_macro_crate::FoundCrate::Itself) => quote! { crate },
        // Not in the dependency list at all. Emitting the canonical name gives the developer a
        // "use of undeclared crate" pointing at the right name to add.
        Err(_) => quote! { ::dry_handlebars },
    }
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
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), String> {
    let content = &assembly.text;
    let struct_name_str = name.replace("-", "_");
    let struct_name = format_ident!("{}", struct_name_str);

    let mut block_map = HashMap::new();
    add_builtins(&mut block_map);

    // The template states its own contract; read it before generating anything.
    let context = context::build(content).map_err(|e| describe(&e, assembly))?;
    let runtime = runtime_crate();
    let types = codegen::generate(&struct_name_str, &context, &runtime);

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
            "{}internal error: dry-handlebars generated invalid Rust for this template. \
             Please report it at https://github.com/paultuckey/dry-handlebars/issues",
            assembly
                .path()
                .map(|path| format!("{}: ", relative(path)))
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

    // Generated items carry documentation so that a consumer denying `missing_docs` needs no
    // `#[allow]`, and so IDE autocomplete says what each setter is for.
    let struct_doc = format!("The `{}` template.", struct_name_str);
    let new_doc = format!(
        "Creates a `{}` from every variable it uses, in the order the template first mentions them.",
        struct_name_str
    );
    let fn_doc = format!(
        "Renders the `{}` template. See `{}_builder` to name the variables instead.",
        struct_name_str, struct_name_str
    );

    let function_def = quote! {
        #[doc = #fn_doc]
        pub fn #method_name<#(#params),*>(#(#names: #field_types),*) -> #struct_name<#(#params),*>
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

            /// Renders the template.
            pub fn render(&self) -> ::std::string::String {
                // Everything here is absolute: generated code must compile whatever the call site
                // has in scope, including a shadowed `String` or `write!`.
                use ::core::fmt::Write as _;
                let mut f = ::std::string::String::new();
                let mut render_inner = || -> ::core::fmt::Result {
                    #render_body
                    ::core::result::Result::Ok(())
                };
                let _ = render_inner();
                f
            }
        }
    };

    Ok((struct_def, function_def))
}

fn generate_code_for_file(
    path: &Path,
    partials: &Path,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), String> {
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
            match generate_code_for_file(path, &root_path) {
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

    let partials = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let (struct_def, function_def) = match generate_code_for_file(&path, &partials) {
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
    // A `str!` template has no directory, so it has nowhere to resolve partials from.
    let assembled = Assembly::build(&content.value(), None, None)
        .and_then(|assembly| generate_code_for_content(&name.value(), &assembly));
    let (struct_def, function_def) = match assembled {
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
