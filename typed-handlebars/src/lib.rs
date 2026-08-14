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
//! [`directory!`] turns each file into a module with a function named after it, taking the
//! template's variables in the order they first appear:
//!
//! ```
//! mod templates {
//!     // The README uses "templates/"; this crate keeps its doc fixtures here.
//!     typed_handlebars::directory!("doc-templates/");
//! }
//!
//! assert_eq!(
//!     templates::button(42, "Save").render(),
//!     r#"<button id="btn42">Save</button>"#
//! );
//! ```
//!
//! Each template also gets a `Builder`, which names each variable rather than relying on argument
//! order, and leaves anything unset empty — as an undefined variable is in Handlebars:
//!
//! ```
//! # mod templates { typed_handlebars::directory!("doc-templates/"); }
//! assert_eq!(
//!     templates::button::Builder::new().btn_id(42).render(),
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
//! `render()` returns a `String`, but a template also implements [`Display`](core::fmt::Display)
//! and exposes `render_to`, so it can be nested inside another template or written straight into a
//! buffer you already have — with no intermediate `String` per level:
//!
//! ```
//! # mod templates { typed_handlebars::directory!("doc-templates/"); }
//! use core::fmt::Write;
//!
//! let mut page = String::from("<div>");
//! templates::button(42, "Save").render_to(&mut page).unwrap();
//! page.push_str("</div>");
//! assert_eq!(page, r#"<div><button id="btn42">Save</button></div>"#);
//! ```
//!
//! `{{ name }}` is HTML-escaped and `{{{ name }}}` is not, as Handlebars specifies. Markup you
//! have already rendered goes in `{{{ }}}`.
//!
//! # Items in this crate
//!
//! Apart from the three macros, everything here — [`Empty`], [`escape`], [`Escaped`], [`Truthy`],
//! [`Set`] and [`IsSet`] — is runtime support that generated code calls into. It is public because
//! the generated code names it, not because you need to: there is nothing here for you to
//! implement.

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
/// beneath it becomes a module named after the file, holding a constructor function, a
/// [`Builder`](crate#the-builder), and whatever types the template implies. Subdirectories become
/// nested modules, so `templates/admin/row.hbs` is `templates::admin::row` and two files called
/// `row.hbs` in different folders do not collide.
///
/// ```
/// mod templates {
///     typed_handlebars::directory!("doc-templates/");
/// }
///
/// // doc-templates/button.hbs and doc-templates/greeting.hbs
/// assert_eq!(
///     templates::button(42, "Save").render(),
///     r#"<button id="btn42">Save</button>"#
/// );
/// assert_eq!(templates::greeting("King").render(), "<p>Hello King!</p>");
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
///     button::button(42, "Save").render(),
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
/// assert_eq!(templates::greeting("King").render(), "<p>Hello King!</p>");
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
///     templates::list::RowsItem::new("King"),
///     templates::list::RowsItem::new("Tubby"),
/// ];
/// assert_eq!(templates::list(rows).render(), "<li>King</li><li>Tubby</li>");
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

/// Wraps a value so that `{{ }}` writes it HTML-escaped.
///
/// Generated code calls this; you should never need to name it. Escaping happens as the value is
/// written, so nothing is allocated on the way.
pub fn escape<T: core::fmt::Display + ?Sized>(value: &T) -> Escaped<'_, T> {
    Escaped(value)
}

/// The HTML-escaping wrapper produced by [`escape`].
pub struct Escaped<'a, T: ?Sized>(&'a T);

impl<T: core::fmt::Display + ?Sized> core::fmt::Display for Escaped<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use core::fmt::Write;
        write!(EscapeWriter(f), "{}", self.0)
    }
}

/// Escapes as it forwards, so a large value never lands in a temporary buffer.
struct EscapeWriter<'a, 'b>(&'a mut core::fmt::Formatter<'b>);

impl core::fmt::Write for EscapeWriter<'_, '_> {
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
