//! Turns the shape a template declares into Rust types.
//!
//! [`crate::parser::context`] reads an `.hbs` file and works out what data it needs; this module
//! emits the structs for it. A template like
//!
//! ```handlebars
//! <h1>{{ title }}</h1>
//! {{#each rows}}<li>{{ name }}</li>{{/each}}
//! ```
//!
//! becomes a `page_rows_item` holding `name`, and a `page` holding `title` and `rows`. The Rust
//! developer names no types and implements no traits — they fill in generated fields.
//!
//! # Generic parameters
//!
//! Fields stay generic so the caller can pass whatever they already have: `&str`, `String`, `u32`
//! or anything else that implements `Display`. Every leaf in the tree contributes one parameter,
//! and parameters from nested types are threaded up into their parent, so the root type carries
//! the whole tree's parameters and the root `impl` carries the whole tree's bounds.
//!
//! Sequences are bound by `for<'a> &'a I: IntoIterator<Item = &'a Item>`, which is what lets
//! `render(&self)` walk a list without consuming it — and lets a template iterate the same list
//! twice.

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::parser::context::{Context, Field, FieldKind};

/// A generated type: what to declare it with, and what its parent needs to know about it.
struct Shape {
    /// Every generic parameter in this subtree, in template order.
    params: Vec<Ident>,
    /// The subset of `params` that appears in this type's own fields.
    ///
    /// A list's item type is named only in that list's `AsRef` bound, so its parameters need a
    /// `PhantomData` to count as used — this is how we tell which ones.
    field_params: Vec<Ident>,
    /// Parameters that end up written out, so they need `Display`.
    display_params: Vec<Ident>,
    /// `where` predicates this subtree needs, hoisted to the root `impl`.
    predicates: Vec<TokenStream>,
    /// `pub name: Type` for each field.
    declarations: Vec<TokenStream>,
    /// Field names, for constructors and call sites.
    names: Vec<Ident>,
    /// Field types, for constructor arguments.
    types: Vec<TokenStream>,
}

impl Shape {
    fn new() -> Self {
        Self {
            params: Vec::new(),
            field_params: Vec::new(),
            display_params: Vec::new(),
            predicates: Vec::new(),
            declarations: Vec::new(),
            names: Vec::new(),
            types: Vec::new(),
        }
    }

    /// The `PhantomData` this type needs, if any parameter is only named in a bound.
    fn phantom(&self) -> Option<TokenStream> {
        let unused: Vec<&Ident> = self
            .params
            .iter()
            .filter(|param| !self.field_params.contains(param))
            .collect();
        if unused.is_empty() {
            None
        } else {
            Some(quote! { ::std::marker::PhantomData<(#(#unused,)*)> })
        }
    }

    /// Field declarations including the marker, ready to drop into a struct body.
    fn all_declarations(&self) -> Vec<TokenStream> {
        let mut declarations = self.declarations.clone();
        if let Some(phantom) = self.phantom() {
            declarations.push(quote! { #MARKER: #phantom });
        }
        declarations
    }

    /// Field initialisers including the marker, ready to drop into `Self { … }`.
    fn all_initialisers(&self) -> Vec<TokenStream> {
        let mut initialisers: Vec<TokenStream> =
            self.names.iter().map(|name| quote! { #name }).collect();
        if self.phantom().is_some() {
            initialisers.push(quote! { #MARKER: ::std::marker::PhantomData });
        }
        initialisers
    }
}

/// Name of the generated marker field.
///
/// Deliberately unlikely to collide with anything an `.hbs` author would write, and private, so it
/// stays invisible: callers go through `new` (and, once todo.md item 2 lands, the builder).
struct Marker;

impl quote::ToTokens for Marker {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        format_ident!("__dry_handlebars_marker").to_tokens(tokens);
    }
}

const MARKER: Marker = Marker;

/// Names generic parameters `T0`, `T1`, … across one template so they never collide.
struct Counter(usize);

impl Counter {
    fn next(&mut self, prefix: &str) -> Ident {
        let ident = format_ident!("{}{}", prefix, self.0);
        self.0 += 1;
        ident
    }
}

/// Renders a `where` clause, or nothing when there are no predicates.
pub fn where_clause(predicates: &[TokenStream]) -> TokenStream {
    if predicates.is_empty() {
        quote! {}
    } else {
        quote! { where #(#predicates),* }
    }
}

/// Everything a template expands to, apart from its render body.
pub struct Types {
    /// Types for nested scopes, declared alongside the template's own type.
    pub nested: Vec<TokenStream>,
    /// Generic parameters of the template's own type.
    pub params: Vec<Ident>,
    /// Bounds for the `impl` block that carries `render`.
    pub predicates: Vec<TokenStream>,
    /// Field declarations of the template's own type, including any marker.
    pub declarations: Vec<TokenStream>,
    /// Field initialisers for `new`, including any marker.
    pub initialisers: Vec<TokenStream>,
    /// Field names, in template order.
    pub names: Vec<Ident>,
    /// Field types, in template order.
    pub types: Vec<TokenStream>,
}

/// Builds the types for a template.
///
/// `mappings` is the Rust-side escape hatch: a name declared there keeps the caller's own type
/// instead of getting a generated one, so existing domain structs can be wired in directly. It
/// applies to top-level fields only — nested shapes always come from the template.
pub fn generate(
    root_name: &str,
    context: &Context,
    mappings: &HashMap<String, syn::Type>,
) -> Types {
    let mut counter = Counter(0);
    let mut nested = Vec::new();
    let shape = build(root_name, context, mappings, &mut counter, &mut nested);

    let declarations = shape.all_declarations();
    let initialisers = shape.all_initialisers();

    let mut predicates = shape.predicates;
    for param in &shape.display_params {
        predicates.push(quote! { #param: ::std::fmt::Display });
    }

    Types {
        nested,
        params: shape.params,
        predicates,
        declarations,
        initialisers,
        names: shape.names,
        types: shape.types,
    }
}

fn build(
    prefix: &str,
    context: &Context,
    mappings: &HashMap<String, syn::Type>,
    counter: &mut Counter,
    nested: &mut Vec<TokenStream>,
) -> Shape {
    let mut shape = Shape::new();

    for field in &context.fields {
        let name = field_ident(&field.name);

        let ty = if let Some(mapped) = mappings.get(&field.name) {
            // The caller supplied a type; the template's inferred shape is only a cross-check.
            quote! { #mapped }
        } else {
            field_type(prefix, field, counter, nested, &mut shape)
        };

        shape.declarations.push(quote! { pub #name: #ty });
        shape.names.push(name);
        shape.types.push(ty);
    }

    shape
}

/// Works out the type of one field, declaring any nested types it needs along the way.
fn field_type(
    prefix: &str,
    field: &Field,
    counter: &mut Counter,
    nested: &mut Vec<TokenStream>,
    shape: &mut Shape,
) -> TokenStream {
    match &field.kind {
        // A variable that is only ever tested still compiles to `if x { … }`, so it has to be a
        // `bool` for now. Handlebars truthiness is todo.md item 3.
        FieldKind::Leaf if field.used_as_condition => quote! { bool },

        FieldKind::Leaf => {
            let param = counter.next("T");
            shape.params.push(param.clone());
            shape.field_params.push(param.clone());
            shape.display_params.push(param.clone());
            quote! { #param }
        }

        FieldKind::Object(inner) => {
            let type_name = format_ident!("{}_{}", prefix, field.name);
            let inner_shape = declare(&type_name, inner, counter, nested);
            // The record's type is named in this field, so its parameters are used here too.
            let params = absorb(shape, inner_shape, true);
            quote! { #type_name<#(#params),*> }
        }

        FieldKind::Sequence(item) => {
            let item_type = if item.is_scalar() {
                // `{{#each tags}}{{this}}{{/each}}` — the items are values, not records.
                let param = counter.next("T");
                shape.params.push(param.clone());
                shape.display_params.push(param.clone());
                quote! { #param }
            } else {
                let type_name = format_ident!("{}_{}_item", prefix, field.name);
                let inner_shape = declare(&type_name, item, counter, nested);
                // The item's type is named only in the bound below, never in a field, so its
                // parameters need the marker.
                let params = absorb(shape, inner_shape, false);
                quote! { #type_name<#(#params),*> }
            };

            let param = counter.next("I");
            shape.params.push(param.clone());
            shape.field_params.push(param.clone());
            shape
                .predicates
                .push(quote! { #param: ::std::convert::AsRef<[#item_type]> });
            quote! { #param }
        }
    }
}

/// Declares a nested type and returns its shape.
fn declare(
    type_name: &Ident,
    context: &Context,
    counter: &mut Counter,
    nested: &mut Vec<TokenStream>,
) -> Shape {
    // Nested types never take the Rust-side mapping escape hatch: their shape comes from the
    // template, which is the only place that describes them.
    let shape = build(
        &type_name.to_string(),
        context,
        &HashMap::new(),
        counter,
        nested,
    );

    let params = &shape.params;
    let names = &shape.names;
    let types = &shape.types;
    let declarations = shape.all_declarations();
    let initialisers = shape.all_initialisers();
    let where_clause = where_clause(&shape.predicates);

    nested.push(quote! {
        pub struct #type_name<#(#params),*> #where_clause {
            #(#declarations),*
        }

        impl<#(#params),*> #type_name<#(#params),*> #where_clause {
            pub fn new(#(#names: #types),*) -> Self {
                Self { #(#initialisers),* }
            }
        }
    });

    shape
}

/// Threads a nested type's parameters and bounds up into its parent.
///
/// `in_field` says whether the parent names the child's type in one of its own fields; if it only
/// appears in a bound, the child's parameters need the parent's `PhantomData`.
fn absorb(parent: &mut Shape, child: Shape, in_field: bool) -> Vec<Ident> {
    parent.params.extend(child.params.iter().cloned());
    if in_field {
        parent.field_params.extend(child.params.iter().cloned());
    }
    parent.display_params.extend(child.display_params);
    parent.predicates.extend(child.predicates);
    child.params
}

/// Maps a template variable name to a Rust identifier.
///
/// Handlebars names that happen to be Rust keywords become raw identifiers: the person writing the
/// template has no reason to know what `type` or `match` mean to a Rust compiler.
fn field_ident(name: &str) -> Ident {
    let sanitised = name.replace('-', "_");
    match syn::parse_str::<Ident>(&sanitised) {
        Ok(ident) => ident,
        Err(_) => Ident::new_raw(&sanitised, proc_macro2::Span::call_site()),
    }
}
