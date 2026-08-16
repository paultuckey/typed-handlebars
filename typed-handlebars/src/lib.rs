//! Compile-time checked [Handlebars](https://handlebarsjs.com/) templates for Rust.
//!
//! Your `.hbs` files are turned into Rust when the crate is built, so there is no parsing, no
//! template registry and no lookups at run time. The types a template needs are generated from
//! what the template itself says, so there is nothing to declare, derive or implement.
//!
//! See the [project README](https://github.com/paultuckey/typed-handlebars#readme) for the goals,
//! the full table of supported Handlebars constructs, and worked examples of partials and nesting.
//!
//! # Example
//!
//! Given `templates/button.hbs`:
//!
//! ```handlebars
//! <button id="btn{{ btn_id }}">{{ btn_name }}</button>
//! ```
//!
//! [`directory!`] turns each file into a module holding a `Vars` — every variable the template
//! uses, named. It is an ordinary struct, so you write it as a literal:
//!
//! ```
//! mod templates {
//!     // The README uses "templates/"; this crate keeps its doc fixtures here.
//!     typed_handlebars::directory!("doc-templates/");
//! }
//!
//! assert_eq!(
//!     templates::button::Vars { btn_id: 42, btn_name: "Save" }.render(),
//!     r#"<button id="btn42">Save</button>"#
//! );
//! ```
//!
//! Nothing depends on argument order, your IDE offers the names, and the compiler checks them: a
//! misspelled field names the one you meant, and a variable added to the `.hbs` breaks every call
//! site rather than quietly rendering as nothing.
//!
//! When you do not have every variable, `builder()` sets the ones you do have and leaves the rest
//! empty — as an undefined variable is in Handlebars:
//!
//! ```
//! # mod templates { typed_handlebars::directory!("doc-templates/"); }
//! assert_eq!(
//!     templates::button::builder().btn_id(42).render(),
//!     r#"<button id="btn42"></button>"#
//! );
//! ```
//!
//! # Entry points
//!
//! - [`directory!`] — a module per `.hbs` file in a folder, mirroring the directory layout.
//!   Resolves `{{> partials}}` against that tree.
//! - [`file!`] — a single template file.
//! - [`str!`] — a template written inline, for a one-liner or a test. No directory, so no partials.
//!
//! A mistake in a template is reported against the `.hbs` file with a line and column, in
//! Handlebars terms; anything outside the supported subset is a compile error naming the
//! construct, never a silent difference in output.
//!
//! # Rendering
//!
//! `render()` returns a `String`, and `render_to` writes into any [`fmt::Write`](core::fmt::Write)
//! sink, so a buffer you already have needs no throwaway `String`:
//!
//! ```
//! # mod templates { typed_handlebars::directory!("doc-templates/"); }
//! use core::fmt::Write;
//!
//! let mut page = String::from("<div>");
//! templates::button::Vars { btn_id: 42, btn_name: "Save" }
//!     .render_to(&mut page)
//!     .unwrap();
//! page.push_str("</div>");
//! assert_eq!(page, r#"<div><button id="btn42">Save</button></div>"#);
//! ```
//!
//! `{{ name }}` is HTML-escaped and `{{{ name }}}` is not, as Handlebars specifies. Markup you
//! have already rendered goes in `{{{ }}}` — which is how one template's output is nested inside
//! another, exactly as handlebars.js passes a rendered fragment in as a variable.
//!
//! A variable can be an `Option`, and `None` writes nothing — as null and undefined do in
//! handlebars.js — so a nullable column needs no unwrapping on the way in:
//!
//! ```
//! # mod templates { typed_handlebars::directory!("doc-templates/"); }
//! let missing: Option<&str> = None;
//! assert_eq!(
//!     templates::button::Vars { btn_id: 42, btn_name: missing }.render(),
//!     r#"<button id="btn42"></button>"#
//! );
//! ```
//!
//! # Items in this crate
//!
//! Apart from the three macros, everything here — [`Empty`], [`Absent`], [`Render`],
//! [`RenderExt`], [`Escaped`], [`Shown`], [`Truthy`], [`Length`], [`Set`] and [`IsSet`] — is
//! runtime support that generated code calls into. It is public because the generated code names
//! it, not because you need to: there is nothing here for you to implement.

// This crate contains no unsafe code, and generated code never emits any.
#![forbid(unsafe_code)]
// Every public item here is named by generated code, so a consumer denying `missing_docs` sees
// these in their own docs — they are documented for that reader, not for this one.
#![warn(missing_docs)]

// Generated code names this crate absolutely, as `::typed_handlebars`, so that one emitted path
// works everywhere: in a consumer's crate, in this crate's own unit tests, and in the doctests
// above — which rustdoc compiles as separate crates depending on this one.
extern crate self as typed_handlebars;

/// Generates a module per `.hbs` file in a directory, mirroring the directory layout.
///
/// The path is relative to the crate root (the directory holding `Cargo.toml`). Every `.hbs` file
/// beneath it becomes a module named after the file, holding a `Vars`, a `builder()`, and whatever
/// types the template implies. Subdirectories become nested modules, so `templates/admin/row.hbs`
/// is `templates::admin::row` and two files called `row.hbs` in different folders do not collide.
///
/// ```
/// mod templates {
///     typed_handlebars::directory!("doc-templates/");
/// }
///
/// // doc-templates/button.hbs and doc-templates/greeting.hbs
/// assert_eq!(
///     templates::button::Vars { btn_id: 42, btn_name: "Save" }.render(),
///     r#"<button id="btn42">Save</button>"#
/// );
/// assert_eq!(templates::greeting::Vars { name: "King" }.render(), "<p>Hello King!</p>");
/// ```
///
/// `{{> partial}}` is resolved against this tree at compile time. Editing any template — or any
/// partial it includes — rebuilds the code generated from it.
///
/// One broken template reports itself and the others still compile.
#[doc(inline)]
pub use typed_handlebars_macros::typed_handlebars_directory as directory;

/// Generates a module for a single `.hbs` file.
///
/// The path is relative to the crate root. Partials are resolved against the file's own directory.
///
/// ```
/// mod button {
///     typed_handlebars::file!("doc-templates/button.hbs");
/// }
///
/// assert_eq!(
///     button::button::Vars { btn_id: 42, btn_name: "Save" }.render(),
///     r#"<button id="btn42">Save</button>"#
/// );
/// ```
///
/// Reach for [`directory!`] unless you want one specific file; it keeps the module layout and the
/// folder layout the same thing.
#[doc(inline)]
pub use typed_handlebars_macros::typed_handlebars_file as file;

/// Generates a module from a template written inline, given a name and the template text.
///
/// Useful for a one-liner or a test. There is no directory to resolve against, so `{{> partial}}`
/// is a compile error here — use [`directory!`] or [`file!`] for templates that include others.
///
/// ```
/// mod templates {
///     typed_handlebars::str!("greeting", "<p>Hello {{ name }}!</p>");
/// }
///
/// assert_eq!(templates::greeting::Vars { name: "King" }.render(), "<p>Hello King!</p>");
/// ```
///
/// The generated types come from the template just as they do for a file, so a list still
/// generates its item type:
///
/// ```
/// mod templates {
///     typed_handlebars::str!("list", "{{#each rows}}<li>{{ name }}</li>{{/each}}");
/// }
///
/// let rows = vec![
///     templates::list::RowsItem { name: "King" },
///     templates::list::RowsItem { name: "Tubby" },
/// ];
/// assert_eq!(templates::list::Vars { rows }.render(), "<li>King</li><li>Tubby</li>");
/// ```
#[doc(inline)]
pub use typed_handlebars_macros::typed_handlebars_str as str;

/// A variable that was never given a value.
///
/// Handlebars treats an undefined variable as empty, and so does this: `Empty` writes nothing when
/// displayed, and stands in for a list with no items. Generated code uses it; you should never need
/// to name it.
pub struct Empty;

impl core::fmt::Display for Empty {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

impl<T> AsRef<[T]> for Empty {
    fn as_ref(&self) -> &[T] {
        &[]
    }
}

/// A list variable that was never given a value.
///
/// [`Empty`] would do, but a list has to name its item type or nothing can infer it, so this is
/// `Empty` with the item type written down. Generated code uses it; you should never need to name
/// it.
///
/// Absent is not the same as empty, and `{{ rows.length }}` is the one place the difference shows:
/// a list that was never set counts as nothing, where a list with no items in it counts `0`. That
/// is what handlebars.js does with an undefined value against an empty array.
pub struct Absent<T>(core::marker::PhantomData<T>);

impl<T> Absent<T> {
    /// Creates the absent list.
    pub fn new() -> Self {
        Absent(core::marker::PhantomData)
    }
}

impl<T> Default for Absent<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AsRef<[T]> for Absent<T> {
    fn as_ref(&self) -> &[T] {
        &[]
    }
}

/// How many items `{{ rows.length }}` reports.
///
/// This follows handlebars.js, where `length` is an ordinary property lookup and a JS array carries
/// one. Only lists have it here: a `String` deliberately does not, because JS counts UTF-16 code
/// units and Rust would count either bytes or `char`s — all three disagree on the same text, and a
/// quietly different number is exactly what this crate promises never to produce.
///
/// [`Count`](Length::Count) is an associated type rather than a plain `usize` so that a list which
/// was never set can report nothing at all, as an undefined value does in handlebars.js, while a
/// list with no items in it reports `0`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no `.length` for a template to count",
    label = "this value is not a list",
    // Doubled braces: this attribute reads `{…}` as a placeholder, as `format!` does.
    note = "`{{{{ x.length }}}}` counts a list — a `Vec`, a slice or an array. A `String` has no \
            `.length` here: JS counts UTF-16 code units and Rust counts bytes or `char`s, so any \
            answer would silently disagree with handlebars.js"
)]
pub trait Length {
    /// What the count renders as: a number, or nothing at all when the list was never set.
    type Count: core::fmt::Display + Truthy;

    /// How many items this holds.
    fn length(&self) -> Self::Count;
}

impl<T> Length for [T] {
    type Count = usize;
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T, const N: usize> Length for [T; N] {
    type Count = usize;
    fn length(&self) -> usize {
        N
    }
}

impl<T> Length for Vec<T> {
    type Count = usize;
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T: Length + ?Sized> Length for &T {
    type Count = T::Count;
    fn length(&self) -> T::Count {
        (**self).length()
    }
}

/// A variable that was never set is absent, and absent has no count — `{{ rows.length }}` writes
/// nothing, rather than `0`, exactly as it does for an undefined value in handlebars.js.
impl Length for Empty {
    type Count = Empty;
    fn length(&self) -> Empty {
        Empty
    }
}

impl<T> Length for Absent<T> {
    type Count = Empty;
    fn length(&self) -> Empty {
        Empty
    }
}

/// How a value is written by `{{ }}` and `{{{ }}}`.
///
/// Anything that implements [`Display`](core::fmt::Display) is written as it displays. `Option` is
/// the exception, and the reason this trait exists rather than a plain `Display` bound: handlebars.js
/// writes nothing at all for a value that is null or undefined, so `None` writes nothing here too —
/// exactly as [`Empty`] does for a variable that was never set.
///
/// `K` says *which* of those routes a value took. It is a marker type, filled in by inference and
/// never written by hand: `Option<T>` cannot go through `Display`, and everything else cannot go
/// through the `Option` impl, so exactly one route ever fits. It has to be a type parameter rather
/// than one blanket impl with a special case inside, because Rust's coherence rules forbid a crate
/// from writing both `impl<T: Display> Render for T` and `impl<T> Render for Option<T>`.
///
/// Every type you would reasonably pass already implements this — there is nothing here for you to
/// write.
// Left to itself, a value that cannot be written reports a missing `Render<ViaDisplay>`, naming a
// marker the caller never wrote and a trait they have no reason to know. The template asked for
// something printable, so that is what the error says.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be written out by a template",
    label = "this value has no text to write",
    note = "a template writes anything that implements `std::fmt::Display`, or an `Option` of \
            such a type — where `None` writes nothing, as it does in handlebars.js"
)]
pub trait Render<K> {
    /// Writes this value into `out`, unescaped.
    fn render_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result;
}

/// [`Render`] marker: written as it displays.
pub struct ViaDisplay;

/// [`Render`] marker: written when `Some`, nothing when `None`.
pub struct ViaOption;

/// [`Render`] marker: as [`ViaOption`], for an `Option` passed by reference.
pub struct ViaOptionRef;

impl<T: core::fmt::Display + ?Sized> Render<ViaDisplay> for T {
    fn render_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result {
        write!(out, "{}", self)
    }
}

/// `None` is absent, and absent writes nothing — as null and undefined do in handlebars.js.
impl<T: core::fmt::Display> Render<ViaOption> for Option<T> {
    fn render_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result {
        match self {
            Some(value) => write!(out, "{}", value),
            None => Ok(()),
        }
    }
}

/// Borrowing the `Option` rather than passing it in is the common case when the value lives in a
/// struct the caller still owns, so it renders the same way.
impl<T: core::fmt::Display> Render<ViaOptionRef> for &Option<T> {
    fn render_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result {
        match self {
            Some(value) => write!(out, "{}", value),
            None => Ok(()),
        }
    }
}

/// Turns a value into something `write!` can take, escaped or not.
///
/// Generated code calls these as methods — `value.escaped()` rather than `escaped(value)` — because
/// method lookup steps through references for us. A loop hands its body an `&Item` while a field is
/// a plain value, and both have to reach the same [`Render`] impl.
pub trait RenderExt<K>: Render<K> {
    /// Wraps this value for `{{{ }}}`: written exactly as given.
    fn shown(&self) -> Shown<'_, Self, K> {
        Shown(self, core::marker::PhantomData)
    }

    /// Wraps this value for `{{ }}`: written HTML-escaped.
    fn escaped(&self) -> Escaped<'_, Self, K> {
        Escaped(self, core::marker::PhantomData)
    }
}

impl<T: Render<K> + ?Sized, K> RenderExt<K> for T {}

/// The wrapper produced by [`RenderExt::shown`].
pub struct Shown<'a, T: ?Sized, K>(&'a T, core::marker::PhantomData<K>);

impl<T: Render<K> + ?Sized, K> core::fmt::Display for Shown<'_, T, K> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.render_to(f)
    }
}

/// The HTML-escaping wrapper produced by [`RenderExt::escaped`].
pub struct Escaped<'a, T: ?Sized, K>(&'a T, core::marker::PhantomData<K>);

impl<T: Render<K> + ?Sized, K> core::fmt::Display for Escaped<'_, T, K> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.render_to(&mut EscapeWriter(f))
    }
}

/// Escapes as it forwards, so a large value never lands in a temporary buffer.
struct EscapeWriter<'a, W: ?Sized>(&'a mut W);

impl<W: core::fmt::Write + ?Sized> core::fmt::Write for EscapeWriter<'_, W> {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        // The same set handlebars.js escapes, so output matches character for character.
        let mut written = 0;
        for (index, character) in text.char_indices() {
            let replacement = match character {
                '&' => "&amp;",
                '<' => "&lt;",
                '>' => "&gt;",
                '"' => "&quot;",
                '\'' => "&#x27;",
                '`' => "&#x60;",
                '=' => "&#x3D;",
                _ => continue,
            };
            self.0.write_str(&text[written..index])?;
            self.0.write_str(replacement)?;
            written = index + character.len_utf8();
        }
        self.0.write_str(&text[written..])
    }
}

/// Whether a value counts as true in `{{#if}}` and `{{#unless}}`.
///
/// This follows handlebars.js: absent, `false`, an empty string, zero and an empty list are all
/// falsy; everything else is truthy. Every type you would reasonably pass already implements it —
/// there is nothing here for you to write.
pub trait Truthy {
    /// Whether `{{#if}}` should render its block for this value.
    fn is_truthy(&self) -> bool;
}

impl Truthy for bool {
    fn is_truthy(&self) -> bool {
        *self
    }
}

/// A variable that was never set is absent, and absent is falsy.
impl Truthy for Empty {
    fn is_truthy(&self) -> bool {
        false
    }
}

/// A list that was never set is absent, and so is falsy — as an empty list is.
impl<T> Truthy for Absent<T> {
    fn is_truthy(&self) -> bool {
        false
    }
}

impl Truthy for str {
    fn is_truthy(&self) -> bool {
        !self.is_empty()
    }
}

impl Truthy for String {
    fn is_truthy(&self) -> bool {
        !self.is_empty()
    }
}

/// `None` is absent; `Some` is present whatever it wraps, as in handlebars.js.
impl<T> Truthy for Option<T> {
    fn is_truthy(&self) -> bool {
        self.is_some()
    }
}

impl<T> Truthy for [T] {
    fn is_truthy(&self) -> bool {
        !self.is_empty()
    }
}

impl<T, const N: usize> Truthy for [T; N] {
    fn is_truthy(&self) -> bool {
        N != 0
    }
}

impl<T> Truthy for Vec<T> {
    fn is_truthy(&self) -> bool {
        !self.is_empty()
    }
}

impl<T: Truthy + ?Sized> Truthy for &T {
    fn is_truthy(&self) -> bool {
        (**self).is_truthy()
    }
}

macro_rules! truthy_if_nonzero {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Truthy for $ty {
                fn is_truthy(&self) -> bool {
                    *self != 0 as $ty
                }
            }
        )*
    };
}

truthy_if_nonzero!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

/// A value a builder has been given.
///
/// Generated code uses this; you should never need to name it.
pub struct Set<T>(pub T);

/// Supplies the value held in a builder slot.
///
/// Every generated builder starts with each slot held by a `<template>_unset_<variable>` marker,
/// which resolves to whatever absent means for that variable — nothing to display, a list with no
/// items, a false condition. Setting a variable swaps the slot for [`Set`]. Nothing here needs
/// naming from outside generated code.
pub trait IsSet {
    /// The type of the value in this slot.
    type Value;

    /// Unwraps the value.
    fn into_value(self) -> Self::Value;
}

impl<T> IsSet for Set<T> {
    type Value = T;

    fn into_value(self) -> T {
        self.0
    }
}
