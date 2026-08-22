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
//! becomes a `RowsItem` holding `name`, and a `Vars` holding `title` and `rows`. The Rust developer
//! names no types and implements no traits — they fill in generated fields.
//!
//! # Plain structs, so a struct literal works
//!
//! Every generated type is plain data: public fields, no hidden field, and no `where` clause. That
//! is what lets a caller write the type out directly —
//!
//! ```ignore
//! templates::page::Vars { title: "Dub", rows: &rows }
//! ```
//!
//! — and get the compiler's own diagnostics for it: a misspelled field is `E0560` with a
//! "did you mean", a forgotten one is `E0063` naming it, a repeated one is `E0062`. Being
//! exhaustive is the point: a variable added to the `.hbs` breaks every call site rather than
//! quietly rendering as nothing. The builder is the way to set only some of them.
//!
//! # Generic parameters
//!
//! Fields stay generic so the caller can pass whatever they already have: `&str`, `String`, `u32`,
//! an `Option` of any of those, or anything else that implements `Display`. A type declares one
//! parameter per *field* — not per leaf of the tree — so `Vars<i64, &str>` stays writeable in a
//! signature. A nested record's parameters do thread up into its parent, because the parent names
//! that record's type in one of its own fields.
//!
//! A list is the exception: its item type is named only in a bound, never in a field, so threading
//! its parameters up would leave them unused (`E0392`) and force a `PhantomData` — which would in
//! turn stop the struct being written as a literal. Instead the container parameter stays opaque
//! and the item's parameters are recovered where the bound lives, on the render method.
//!
//! # Why the bounds are on `render`, not on the struct
//!
//! A written value goes through `Render<K>` rather than `Display`, so that an `Option` can write
//! nothing rather than failing to compile. `K` is a marker, filled in by inference, naming which
//! route the value took.
//!
//! Those markers — and every other bound — sit on `render`/`render_to` as method-level generics
//! rather than on the type. Two things follow. The type carries no marker parameters, so
//! `Vars<i64, bool, &String>` is exactly the values and nothing else. And the type needs no
//! `PhantomData` to hold a marker that names no field, which is the second half of what makes a
//! struct literal possible.
//!
//! The cost is `Display`: `Display::fmt` takes no generic parameters, so an `impl Display for Vars`
//! would leave the markers unconstrained (`E0207`). A template value is therefore not itself
//! writeable into another template — nest by passing `inner.render()`, a `String`, exactly as
//! handlebars.js does when it renders a partial's HTML into `{{{ content }}}`.
//!
//! Sequences are bound by `I: AsRef<[Item]>`, which is what lets `render(&self)` walk a list
//! without consuming it — so the caller can pass a list they still own, and a template can iterate
//! the same list twice. `as_ref()` compiles to nothing.
//!
//! # The builder
//!
//! Every value known means a struct literal; anything less means the builder, which is reached
//! through `builder()` beside the type it builds. Handlebars renders an undefined variable as
//! nothing, and only the builder can express that — a struct literal has no way to leave a field
//! out, because `..Default::default()` cannot change a type's parameters on stable Rust. See
//! [`builder_for`].

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::parser::context::{Context, Field, FieldKind};

/// A generated type: what to declare it with, and what its parent needs to know about it.
struct Shape {
    /// The parameters this type declares, in template order — one per field, plus those threaded
    /// up from any nested record.
    params: Vec<Ident>,
    /// What each entry of `params` becomes when its field is left unset.
    ///
    /// Needed because `Empty` is a list of *any* item type, so an unset list has to name a concrete
    /// empty — `Absent<item>` — or the item type cannot be inferred.
    param_empties: Vec<TokenStream>,
    /// Parameters the render method declares rather than the type: `Render` markers, and the
    /// parameters of every list item.
    ///
    /// An item type is named only in its list's `AsRef` bound. Left on the struct those parameters
    /// would be unused and need a `PhantomData`, so they are recovered here instead, where the
    /// bound is.
    method_params: Vec<Ident>,
    /// `where` predicates for the render method, hoisted from the whole subtree.
    predicates: Vec<TokenStream>,
    /// `pub name: Type` for each field.
    declarations: Vec<TokenStream>,
    /// Field names, for constructors and call sites.
    names: Vec<Ident>,
    /// The same names as the template spells them, for building marker type names.
    plain_names: Vec<String>,
    /// Field types, for builder slots.
    types: Vec<TokenStream>,
    /// What each field falls back to when a builder leaves it out — its type and its value.
    empties: Vec<(TokenStream, TokenStream)>,
}

impl Shape {
    fn new() -> Self {
        Self {
            params: Vec::new(),
            param_empties: Vec::new(),
            method_params: Vec::new(),
            predicates: Vec::new(),
            declarations: Vec::new(),
            names: Vec::new(),
            plain_names: Vec::new(),
            types: Vec::new(),
            empties: Vec::new(),
        }
    }
}

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
    /// Every type name this template's module has already spoken for.
    ///
    /// A variable is named by whoever writes the `.hbs`, and `{{ builder.name }}` is ordinary
    /// Handlebars — but it camel-cases straight onto a type this module already generates. Left
    /// alone that is `E0428: the name Builder is defined multiple times`, a wall of Rust errors
    /// against a template that is not wrong about anything.
    taken: HashSet<String>,
}

impl State {
    fn next(&mut self, prefix: &str) -> Ident {
        let ident = format_ident!("{}{}", prefix, self.counter);
        self.counter += 1;
        ident
    }

    /// Mints a type name that nothing else in the module can be using.
    ///
    /// A name that is already spoken for takes a trailing underscore, the same escape
    /// [`crate::sanitise_ident`] gives a Rust keyword, until it is free. Everything the type brings
    /// with it is reserved at the same time — its builder and its unset markers — since those are
    /// derived names that a *later* variable could otherwise land on.
    fn type_name(&mut self, candidate: String, context: &Context) -> Ident {
        let mut name = candidate;
        while !self.claim(&name, context) {
            name.push('_');
        }
        format_ident!("{}", name)
    }

    /// Takes `name` and everything generated alongside it, or reports that something is in the way.
    fn claim(&mut self, name: &str, context: &Context) -> bool {
        let mut wanted = vec![name.to_string(), format!("{}Builder", name)];
        for field in &context.fields {
            wanted.push(format!("{}Unset{}", name, camel(&field.name)));
        }
        if wanted.iter().any(|name| self.taken.contains(name)) {
            return false;
        }
        self.taken.extend(wanted);
        true
    }
}

/// Generates the builder for one type, and the call that starts it.
///
/// A struct literal has to name every field. That is right when the caller has every value, and
/// wrong when they do not — Handlebars renders an undefined variable as nothing, and no struct
/// literal can express that, because `..Default::default()` cannot change a type's parameters on
/// stable Rust. So each type also gets a builder: one named setter per template variable, filled in
/// by autocomplete, with anything left unset rendering empty.
///
/// Every slot starts as a `<type>Unset<Field>` marker, and setting a variable swaps that slot for
/// `typed_handlebars::Set`. A marker resolves to whatever "absent" means for its field — nothing to
/// display, a list with no items, a false condition.
///
/// The exception is a field whose type the caller declared in Rust: there is no empty we can invent
/// for someone else's type, so its marker implements nothing and leaving it out is a compile error
/// naming the variable.
fn builder_for(
    type_name: &Ident,
    builder_name: &Ident,
    shape: &Shape,
    root: bool,
    runtime: &TokenStream,
    frame: &FrameTokens,
) -> TokenStream {
    if shape.names.is_empty() {
        // A template with no variables has nothing to wire up.
        return quote! {};
    }
    let builder_doc = format!(
        "Builds a [`{}`] by naming each variable, so nothing has to be set that you do not have.",
        type_name
    );
    let names = &shape.names;
    let types = &shape.types;
    let params = &shape.params;
    let method_params = &shape.method_params;
    let predicates = &shape.predicates;
    let param_empties = &shape.param_empties;

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

    // `build` re-derives the built type's parameters from the slots. Pinning `Value` is what lets
    // them be method-level generics: without it they would be unconstrained. The built type carries
    // no bounds of its own, so these are the whole of what `build` needs.
    let build_bounds = quote! {
        where
            #(#slots: #runtime::IsSet<Value = #types>,)*
    };

    // Only the root renders, and rendering is where every bound in the subtree lives — so its
    // signature carries the render method's own generics on top of the built type's.
    let render = if root {
        let frame_param = &frame.param;
        let frame_argument = &frame.argument;
        quote! {
            /// Renders straight from the builder.
            pub fn render<#(#params,)* #(#method_params),*>(
                self,
                #frame_param
            ) -> ::std::string::String
            where
                #(#slots: #runtime::IsSet<Value = #types>,)*
                #(#predicates,)*
            {
                let built: #type_name<#(#params),*> = self.build();
                built.render(#frame_argument)
            }
        }
    } else {
        quote! {}
    };

    // How the builder is reached. At the root that is a free function beside the type, which is the
    // shortest path to the thing a caller wants; a nested type hangs it off the type itself, the
    // way `builder()` is conventionally found in Rust.
    let entry_doc = format!(
        "Starts a [`{}`] with every variable unset. Anything left unset renders empty.",
        type_name
    );
    let entry = if root {
        quote! {
            #[doc = #entry_doc]
            pub fn builder() -> #builder_name<#(#unset),*> {
                #builder_name::new()
            }
        }
    } else {
        quote! {
            impl #type_name<#(#param_empties),*> {
                #[doc = #entry_doc]
                pub fn builder() -> #builder_name<#(#unset),*> {
                    #builder_name::new()
                }
            }
        }
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

            /// Builds the value, once every variable you have has been set.
            pub fn build<#(#params),*>(self) -> #type_name<#(#params),*> #build_bounds {
                #type_name {
                    #(#names: #runtime::IsSet::into_value(self.#names)),*
                }
            }

            #render
        }

        #entry
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
    /// The template's builder, and the `builder()` that starts it.
    pub builder: TokenStream,
    /// Generic parameters of the template's own type.
    pub params: Vec<Ident>,
    /// Extra generic parameters the render methods declare: markers and list item types.
    pub method_params: Vec<Ident>,
    /// Bounds for the render methods, which is where every bound in the subtree lives.
    pub predicates: Vec<TokenStream>,
    /// Field declarations of the template's own type.
    pub declarations: Vec<TokenStream>,
}

/// How a template's render methods take the frame — empty when it calls no helper.
///
/// Built once by the caller so that the type's own render methods and the builder's shortcut to
/// them agree, since a mismatch would only show up as a Rust error against generated code.
#[derive(Default)]
pub struct FrameTokens {
    /// The parameter, as a signature spells it.
    pub param: TokenStream,
    /// The name to pass it on by.
    pub argument: TokenStream,
}

/// Builds the types for a template.
///
/// Everything comes from the template: it is the only place that describes the data.
pub fn generate(context: &Context, runtime: &TokenStream, frame: &FrameTokens) -> Types {
    let mut state = State {
        counter: 0,
        runtime: runtime.clone(),
        taken: HashSet::new(),
    };
    // The module's own API comes first, so a variable called `vars` or `builder` is the one that
    // gives way rather than the type a caller is meant to write.
    state.claim("Vars", context);
    state.claim("Builder", context);
    let mut nested = Vec::new();
    // Nested types are named from their path through the template — `RowsItemCellsItem` — and the
    // template's own module keeps them from colliding with anything else.
    let shape = build("", context, &mut state, &mut nested);

    let builder = builder_for(
        &format_ident!("Vars"),
        &format_ident!("Builder"),
        &shape,
        true,
        &state.runtime,
        frame,
    );

    Types {
        nested,
        builder,
        params: shape.params,
        method_params: shape.method_params,
        predicates: shape.predicates,
        declarations: shape.declarations,
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

/// Records how a written value reaches the page.
///
/// The marker names no field and is filled in by inference, so it belongs on the render method
/// rather than on the type — which is what keeps the type free of `PhantomData`.
fn render_bound(param: &Ident, state: &mut State, shape: &mut Shape) {
    let marker = state.next("K");
    let runtime = &state.runtime;
    shape
        .predicates
        .push(quote! { #param: #runtime::Render<#marker> });
    shape.method_params.push(marker);
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
            bounds_for_use(&param, field.into(), state, shape);
            shape
                .empties
                .push((empty_type(&runtime), empty_type(&runtime)));
            quote! { #param }
        }

        FieldKind::Object(inner) => {
            let type_name = state.type_name(format!("{}{}", prefix, camel(&field.name)), inner);
            let doc = format!("The `{}` record.", field.name);
            let inner_shape = declare(&type_name, &doc, inner, state, nested);

            // An unset record is one with every field empty.
            shape.empties.push(inner_empty(&type_name, &inner_shape));

            // The record's type is named in this field, so its parameters belong to this type too.
            let params = inner_shape.params.clone();
            absorb(shape, inner_shape);
            quote! { #type_name<#(#params),*> }
        }

        FieldKind::Sequence(item) => {
            let (item_type, empty_item) = if item.is_scalar() {
                // `{{#each tags}}{{this}}{{/each}}` — the items are values, not records. The item
                // type is named only in the `AsRef` bound below, so its parameter goes where that
                // bound goes: onto the render method.
                let param = state.next("T");
                shape.method_params.push(param.clone());
                bounds_for_use(&param, item.into(), state, shape);
                (quote! { #param }, empty_type(&runtime))
            } else {
                let type_name =
                    state.type_name(format!("{}{}Item", prefix, camel(&field.name)), item);
                let doc = format!("One item of the `{}` list.", field.name);
                let inner_shape = declare(&type_name, &doc, item, state, nested);
                let empty_item = {
                    let empties = &inner_shape.param_empties;
                    quote! { #type_name<#(#empties),*> }
                };
                let params = inner_shape.params.clone();
                // Same reasoning as a scalar item: the item type appears in no field of this type,
                // so its parameters are the render method's rather than the struct's.
                shape.method_params.extend(params.iter().cloned());
                shape.method_params.extend(inner_shape.method_params);
                shape.predicates.extend(inner_shape.predicates);
                (quote! { #type_name<#(#params),*> }, empty_item)
            };

            let param = state.next("I");
            shape.params.push(param.clone());
            shape
                .param_empties
                .push(quote! { #runtime::Absent<#empty_item> });
            shape
                .predicates
                .push(quote! { #param: ::std::convert::AsRef<[#item_type]> });
            if field.used_as_condition {
                shape.predicates.push(quote! { #param: #runtime::Truthy });
            }
            if field.used_as_length {
                shape.predicates.push(quote! { #param: #runtime::Length });
            }
            // An unset list is absent rather than empty, which `{{ rows.length }}` is the one
            // place to notice: absent counts as nothing where an empty list counts `0`. It has to
            // name the item type — `Empty` is a list of anything, which would leave the item type
            // ambiguous — which is what `Absent` is for.
            shape.empties.push((
                quote! { #runtime::Absent<#empty_item> },
                quote! { #runtime::Absent::new() },
            ));
            quote! { #param }
        }
    }
}

/// Adds the bounds a value earns from what the template does with it.
///
/// Printed needs `Render`, tested needs `Truthy`, counted needs `Length` — and testing a variable
/// never stops you printing it, so these are not exclusive. A tested variable takes Handlebars
/// truthiness rather than being forced to `bool`, which is what lets
/// `{{#if title}}{{title}}{{/if}}` work on the very string it prints.
fn bounds_for_use(param: &Ident, use_: Uses, state: &mut State, shape: &mut Shape) {
    let runtime = state.runtime.clone();
    if use_.as_value {
        render_bound(param, state, shape);
    }
    if use_.as_condition {
        shape.predicates.push(quote! { #param: #runtime::Truthy });
    }
    // `{{ rows.length }}` with no `{{#each rows}}` anywhere: the template says `rows` is a list,
    // but never says what is in it, so `Length` is the whole of what it needs.
    if use_.as_length {
        shape.predicates.push(quote! { #param: #runtime::Length });
    }
}

/// What a template does with one value.
///
/// A named field and a `{{#each}}` item earn their bounds the same way, but the parser describes
/// them with two different types, so this is the part both have in common.
#[derive(Clone, Copy)]
struct Uses {
    as_value: bool,
    as_condition: bool,
    as_length: bool,
}

impl From<&Field> for Uses {
    fn from(field: &Field) -> Self {
        Uses {
            as_value: field.used_as_value,
            as_condition: field.used_as_condition,
            as_length: field.used_as_length,
        }
    }
}

impl From<&Context> for Uses {
    fn from(context: &Context) -> Self {
        Uses {
            as_value: context.used_as_value,
            as_condition: context.used_as_condition,
            as_length: context.used_as_length,
        }
    }
}

/// The fallback for a value that is simply absent: nothing to display.
fn empty_type(runtime: &TokenStream) -> TokenStream {
    quote! { #runtime::Empty }
}

/// The fallback for an unset record: the generated type with every field empty.
fn inner_empty(type_name: &Ident, inner: &Shape) -> (TokenStream, TokenStream) {
    let names = &inner.names;
    let values = inner.empties.iter().map(|(_, value)| value);
    let empties = &inner.param_empties;
    (
        quote! { #type_name<#(#empties),*> },
        quote! { #type_name { #(#names: #values),* } },
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
    let declarations = &shape.declarations;
    let builder_name = format_ident!("{}Builder", type_name);
    // A nested type does not render, so it never takes the frame.
    let builder = builder_for(
        type_name,
        &builder_name,
        &shape,
        false,
        &runtime,
        &FrameTokens::default(),
    );

    nested.push(quote! {
        #[doc = #doc]
        pub struct #type_name<#(#params),*> {
            #(#declarations),*
        }

        // A record is present, so `{{#if person}}` renders. This is the one place the crate parts
        // company with handlebars.js: a record left unset is a record of empties rather than an
        // absent one, so it still counts as present.
        impl<#(#params),*> #runtime::Truthy for #type_name<#(#params),*> {
            fn is_truthy(&self) -> bool {
                true
            }
        }

        #builder
    });

    shape
}

/// Threads a nested record's parameters and bounds up into its parent.
///
/// Only records: the parent names a record's type in one of its own fields, so those parameters are
/// genuinely the parent's. A list's item type appears in no field, so it is handled where its bound
/// is instead — see [`field_type`].
fn absorb(parent: &mut Shape, child: Shape) {
    parent.params.extend(child.params);
    parent.param_empties.extend(child.param_empties);
    parent.method_params.extend(child.method_params);
    parent.predicates.extend(child.predicates);
}

/// Maps a template variable name to a Rust identifier.
fn field_ident(name: &str) -> Ident {
    format_ident!("{}", crate::sanitise_ident(name))
}
