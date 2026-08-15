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
//! Sequences are bound by `I: AsRef<[Item]>`, which is what lets `render(&self)` walk a list
//! without consuming it — so the caller can pass a list they still own, and a template can iterate
//! the same list twice. `as_ref()` compiles to nothing.
//!
//! # The builder
//!
//! Each generated type also gets a builder, which is the API a Rust developer actually wires
//! against: one named setter per template variable, filled in by autocomplete rather than by
//! counting positional arguments. See [`builder_for`].

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::parser::context::{Context, Field, FieldKind};

/// A generated type: what to declare it with, and what its parent needs to know about it.
struct Shape {
    /// Every generic parameter in this subtree, in template order.
    params: Vec<Ident>,
    /// What each entry of `params` becomes when its field is left unset.
    ///
    /// Needed because `Empty` is a list of *any* item type, so an unset list has to name a concrete
    /// empty — `[item; 0]` — or the item type cannot be inferred.
    param_empties: Vec<TokenStream>,
    /// The subset of `params` that appears in this type's own fields.
    ///
    /// A list's item type is named only in that list's `AsRef` bound, so its parameters need a
    /// `PhantomData` to count as used — this is how we tell which ones.
    field_params: Vec<Ident>,
    /// Parameters that end up written out, so they need `Display`.
    display_params: Vec<Ident>,
    /// Parameters that are tested by `{{#if}}` or `{{#unless}}`, so they need `Truthy`.
    truthy_params: Vec<Ident>,
    /// `where` predicates this subtree needs, hoisted to the root `impl`.
    predicates: Vec<TokenStream>,
    /// `pub name: Type` for each field.
    declarations: Vec<TokenStream>,
    /// Field names, for constructors and call sites.
    names: Vec<Ident>,
    /// The same names as the template spells them, for building marker type names.
    plain_names: Vec<String>,
    /// Field types, for constructor arguments.
    types: Vec<TokenStream>,
    /// What each field falls back to when a builder leaves it out — its type and its value.
    empties: Vec<(TokenStream, TokenStream)>,
}

impl Shape {
    fn new() -> Self {
        Self {
            params: Vec::new(),
            param_empties: Vec::new(),
            field_params: Vec::new(),
            display_params: Vec::new(),
            truthy_params: Vec::new(),
            predicates: Vec::new(),
            declarations: Vec::new(),
            names: Vec::new(),
            plain_names: Vec::new(),
            types: Vec::new(),
            empties: Vec::new(),
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
/// stays invisible: callers go through the builder, or `new`.
struct Marker;

impl quote::ToTokens for Marker {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        format_ident!("__typed_handlebars_marker").to_tokens(tokens);
    }
}

const MARKER: Marker = Marker;

/// Turns a template variable name into the CamelCase half of a generated type name.
fn camel(name: &str) -> String {
    let mut out = String::new();
    let mut capitalise = true;
    for character in name.chars() {
        if !character.is_alphanumeric() {
            capitalise = true;
            continue;
        }
        if capitalise {
            out.extend(character.to_uppercase());
            capitalise = false;
        } else {
            out.push(character);
        }
    }
    // `self` camel-cases to `Self`, which is still reserved, and a name starting with a digit is
    // still not an identifier.
    crate::sanitise_ident(&out)
}

/// State shared across one template's code generation.
struct State {
    /// Names generic parameters `T0`, `T1`, … so they never collide.
    counter: usize,
    /// How generated code reaches the runtime crate, which depends on what the consumer called it.
    runtime: TokenStream,
}

impl State {
    fn next(&mut self, prefix: &str) -> Ident {
        let ident = format_ident!("{}{}", prefix, self.counter);
        self.counter += 1;
        ident
    }
}

/// Generates the builder for one type: the wiring API.
///
/// Each variable the template uses becomes a named setter, so a developer fills them in by
/// autocomplete rather than by counting positional arguments — and a renamed variable becomes a
/// compile error instead of a silently mis-ordered argument.
///
/// Every slot starts as a `<type>_unset_<field>` marker, and setting a variable swaps that slot for
/// `typed_handlebars::Set`. A marker resolves to whatever "absent" means for its field — nothing to
/// display, a list with no items, a false condition — so a builder need not set everything, which
/// is how Handlebars treats an undefined variable.
///
/// The exception is a field whose type the caller declared in Rust: there is no empty we can invent
/// for someone else's type, so its marker implements nothing and leaving it out is a compile error
/// naming the variable.
fn builder_for(
    type_name: &Ident,
    builder_name: &Ident,
    shape: &Shape,
    renders: bool,
    runtime: &TokenStream,
) -> TokenStream {
    if shape.names.is_empty() {
        // A template with no variables has nothing to wire up.
        return quote! {};
    }
    let builder_doc = format!(
        "Builds a [`{}`] by naming each variable, so nothing depends on argument order.",
        type_name
    );
    let names = &shape.names;
    let types = &shape.types;
    let params = &shape.params;
    let predicates = &shape.predicates;

    let slots: Vec<Ident> = (0..names.len()).map(|i| format_ident!("S{}", i)).collect();
    let unset: Vec<Ident> = shape
        .plain_names
        .iter()
        .map(|name| format_ident!("{}Unset{}", type_name, camel(name)))
        .collect();

    // One setter per variable: it replaces its own slot and passes the others through.
    let setters = names.iter().enumerate().map(|(index, name)| {
        let returned: Vec<TokenStream> = slots
            .iter()
            .enumerate()
            .map(|(slot, ident)| {
                if slot == index {
                    quote! { #runtime::Set<__DhValue> }
                } else {
                    quote! { #ident }
                }
            })
            .collect();
        let moved = names.iter().enumerate().map(|(field, ident)| {
            if field == index {
                quote! { #ident: #runtime::Set(value) }
            } else {
                quote! { #ident: self.#ident }
            }
        });
        let doc = format!(
            "Sets the `{}` variable. Leaving it unset renders it empty.",
            shape.plain_names[index]
        );
        quote! {
            #[doc = #doc]
            pub fn #name<__DhValue>(self, value: __DhValue) -> #builder_name<#(#returned),*> {
                #builder_name { #(#moved),* }
            }
        }
    });

    // `build` re-derives the type's own parameters from the slots. Pinning `Value` is what lets
    // them be method-level generics: without it they would be unconstrained.
    let build_bounds = quote! {
        where
            #(#slots: #runtime::IsSet<Value = #types>,)*
            #(#predicates,)*
    };

    let render = if renders {
        quote! {
            /// Renders straight from the builder.
            pub fn render<#(#params),*>(self) -> ::std::string::String #build_bounds {
                let built: #type_name<#(#params),*> = self.build();
                built.render()
            }
        }
    } else {
        quote! {}
    };

    // What each marker resolves to when its variable is never set.
    let fallbacks = unset
        .iter()
        .zip(&shape.empties)
        .map(|(marker, (ty, value))| {
            quote! {
                impl #runtime::IsSet for #marker {
                    type Value = #ty;
                    fn into_value(self) -> Self::Value { #value }
                }
            }
        });

    quote! {
        #(
            #[doc(hidden)]
            pub struct #unset;
        )*

        #(#fallbacks)*

        #[doc = #builder_doc]
        pub struct #builder_name<#(#slots),*> {
            #(#names: #slots),*
        }

        impl #builder_name<#(#unset),*> {
            #[doc = "Starts with every variable unset. Anything left unset renders empty."]
            pub fn new() -> Self {
                #builder_name { #(#names: #unset),* }
            }
        }

        impl ::std::default::Default for #builder_name<#(#unset),*> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<#(#slots),*> #builder_name<#(#slots),*> {
            #(#setters)*

            /// Builds the template value, once every variable has been set.
            pub fn build<#(#params),*>(self) -> #type_name<#(#params),*> #build_bounds {
                #type_name::new(
                    #(#runtime::IsSet::into_value(self.#names)),*
                )
            }

            #render
        }
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
    /// The template's builder — the wiring API.
    pub builder: TokenStream,
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
/// Everything comes from the template: it is the only place that describes the data.
pub fn generate(context: &Context, runtime: &TokenStream) -> Types {
    let mut state = State {
        counter: 0,
        runtime: runtime.clone(),
    };
    let mut nested = Vec::new();
    // Nested types are named from their path through the template — `RowsItemCellsItem` — and the
    // template's own module keeps them from colliding with anything else.
    let mut shape = build("", context, &mut state, &mut nested);

    let declarations = shape.all_declarations();
    let initialisers = shape.all_initialisers();

    // These belong with the rest, so the builder's `build` carries them too.
    for param in &shape.display_params.clone() {
        shape
            .predicates
            .push(quote! { #param: ::std::fmt::Display });
    }
    for param in &shape.truthy_params.clone() {
        shape.predicates.push(quote! { #param: #runtime::Truthy });
    }

    let builder = builder_for(
        &format_ident!("Template"),
        &format_ident!("Builder"),
        &shape,
        true,
        &state.runtime,
    );

    Types {
        nested,
        builder,
        params: shape.params,
        predicates: shape.predicates,
        declarations,
        initialisers,
        names: shape.names,
        types: shape.types,
    }
}

fn build(
    prefix: &str,
    context: &Context,
    state: &mut State,
    nested: &mut Vec<TokenStream>,
) -> Shape {
    let mut shape = Shape::new();

    for field in &context.fields {
        let name = field_ident(&field.name);
        let ty = field_type(prefix, field, state, nested, &mut shape);

        let doc = format!("The `{}` variable.", field.name);
        shape
            .declarations
            .push(quote! { #[doc = #doc] pub #name: #ty });
        shape.names.push(name);
        shape.plain_names.push(field.name.clone());
        shape.types.push(ty);
    }

    shape
}

/// Works out the type of one field, declaring any nested types it needs along the way.
fn field_type(
    prefix: &str,
    field: &Field,
    state: &mut State,
    nested: &mut Vec<TokenStream>,
    shape: &mut Shape,
) -> TokenStream {
    let runtime = state.runtime.clone();
    match &field.kind {
        FieldKind::Leaf => {
            let param = state.next("T");
            shape.params.push(param.clone());
            shape.param_empties.push(empty_type(&runtime));
            shape.field_params.push(param.clone());
            if field.used_as_value {
                shape.display_params.push(param.clone());
            }
            // A tested variable takes Handlebars truthiness rather than being forced to `bool`,
            // which is what lets `{{#if title}}{{title}}{{/if}}` work on the very string it prints.
            if field.used_as_condition {
                shape.truthy_params.push(param.clone());
            }
            shape
                .empties
                .push((empty_type(&runtime), empty_type(&runtime)));
            quote! { #param }
        }

        FieldKind::Object(inner) => {
            let type_name = format_ident!("{}{}", prefix, camel(&field.name));
            let doc = format!("The `{}` record.", field.name);
            let inner_shape = declare(&type_name, &doc, inner, state, nested);

            // An unset record is one with every field empty.
            shape.empties.push(inner_empty(&type_name, &inner_shape));

            // The record's type is named in this field, so its parameters are used here too.
            let params = absorb(shape, inner_shape, true);
            quote! { #type_name<#(#params),*> }
        }

        FieldKind::Sequence(item) => {
            let (item_type, empty_item) = if item.is_scalar() {
                // `{{#each tags}}{{this}}{{/each}}` — the items are values, not records.
                let param = state.next("T");
                shape.params.push(param.clone());
                shape.param_empties.push(empty_type(&runtime));
                // Bounded by what the template does with the item, exactly as a named field is
                // above: printed needs `Display`, tested needs `Truthy`, and `{{#if this}}` is a
                // test of the item itself rather than of a field on it.
                if item.used_as_value {
                    shape.display_params.push(param.clone());
                }
                if item.used_as_condition {
                    shape.truthy_params.push(param.clone());
                }
                (quote! { #param }, empty_type(&runtime))
            } else {
                let type_name = format_ident!("{}{}Item", prefix, camel(&field.name));
                let doc = format!("One item of the `{}` list.", field.name);
                let inner_shape = declare(&type_name, &doc, item, state, nested);
                let inner_empties = inner_shape.param_empties.clone();
                let empty_item = quote! { #type_name<#(#inner_empties),*> };
                // The item's type is named only in the bound below, never in a field, so its
                // parameters need the marker.
                let params = absorb(shape, inner_shape, false);
                (quote! { #type_name<#(#params),*> }, empty_item)
            };

            let param = state.next("I");
            shape.params.push(param.clone());
            shape.param_empties.push(quote! { [#empty_item; 0] });
            shape.field_params.push(param.clone());
            shape
                .predicates
                .push(quote! { #param: ::std::convert::AsRef<[#item_type]> });
            if field.used_as_condition {
                shape.truthy_params.push(param.clone());
            }
            // An unset list has no items. It has to name the item type: `Empty` is a list of
            // anything, which would leave the item type ambiguous.
            shape
                .empties
                .push((quote! { [#empty_item; 0] }, quote! { [] }));
            quote! { #param }
        }
    }
}

/// The fallback for a value that is simply absent: nothing to display.
fn empty_type(runtime: &TokenStream) -> TokenStream {
    quote! { #runtime::Empty }
}

/// The fallback for an unset record: the generated type with every field empty.
fn inner_empty(type_name: &Ident, inner: &Shape) -> (TokenStream, TokenStream) {
    let values = inner.empties.iter().map(|(_, value)| value);
    let empties = &inner.param_empties;
    (
        quote! { #type_name<#(#empties),*> },
        quote! { #type_name::new(#(#values),*) },
    )
}

/// Declares a nested type and returns its shape.
fn declare(
    type_name: &Ident,
    doc: &str,
    context: &Context,
    state: &mut State,
    nested: &mut Vec<TokenStream>,
) -> Shape {
    let runtime = state.runtime.clone();
    let shape = build(&type_name.to_string(), context, state, nested);

    let params = &shape.params;
    let names = &shape.names;
    let types = &shape.types;
    let declarations = shape.all_declarations();
    let initialisers = shape.all_initialisers();
    let where_clause = where_clause(&shape.predicates);
    let builder_name = format_ident!("{}Builder", type_name);
    let builder = builder_for(type_name, &builder_name, &shape, false, &runtime);

    let new_doc = format!(
        "Creates a `{}` from every variable it uses, in the order the template first mentions them.",
        type_name
    );

    nested.push(quote! {
        #[doc = #doc]
        pub struct #type_name<#(#params),*> #where_clause {
            #(#declarations),*
        }

        impl<#(#params),*> #type_name<#(#params),*> #where_clause {
            #[doc = #new_doc]
            pub fn new(#(#names: #types),*) -> Self {
                Self { #(#initialisers),* }
            }
        }

        // A record is present, so `{{#if person}}` renders. This is the one place the crate parts
        // company with handlebars.js: a record left unset is a record of empties rather than an
        // absent one, so it still counts as present.
        impl<#(#params),*> #runtime::Truthy for #type_name<#(#params),*> #where_clause {
            fn is_truthy(&self) -> bool {
                true
            }
        }

        #builder
    });

    shape
}

/// Threads a nested type's parameters and bounds up into its parent.
///
/// `in_field` says whether the parent names the child's type in one of its own fields; if it only
/// appears in a bound, the child's parameters need the parent's `PhantomData`.
fn absorb(parent: &mut Shape, child: Shape, in_field: bool) -> Vec<Ident> {
    parent.params.extend(child.params.iter().cloned());
    parent.param_empties.extend(child.param_empties);
    if in_field {
        parent.field_params.extend(child.params.iter().cloned());
    }
    parent.display_params.extend(child.display_params);
    parent.truthy_params.extend(child.truthy_params);
    parent.predicates.extend(child.predicates);
    child.params
}

/// Maps a template variable name to a Rust identifier.
fn field_ident(name: &str) -> Ident {
    format_ident!("{}", crate::sanitise_ident(name))
}
