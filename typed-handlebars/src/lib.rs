//! Compile-time checked [Handlebars](https://handlebarsjs.com/) templates for Rust.
//!
//! Your `.hbs` files are the master copy. At build time they become Rust — no parsing, no template
//! registry and no lookups at run time — and the types they need are generated from what the
//! template itself says.
//!
//! This is built for two people:
//!
//! - **Whoever writes the templates needs to know no Rust.** An `.hbs` file is plain Handlebars,
//!   with no type names, annotations or macro-specific syntax, and it renders the same under
//!   handlebars.js. A mistake in a template is reported against the template, with a line and
//!   column.
//! - **Whoever writes the Rust only does the wiring.** There is nothing to derive and no trait to
//!   implement. `{{#each rows}}{{ name }}{{/each}}` says `rows` is a list of records with a
//!   `name`, so that type is generated for you, and your IDE supplies the names instead of you
//!   retyping them from the template.
//!
//! # Getting started
//!
//! Point [`directory!`] at a folder of templates. Given `templates/button.hbs`:
//!
//! ```handlebars
//! <button id="btn{{ btn_id }}">{{ btn_name }}</button>
//! ```
//!
//! each file becomes a module with a function named after it. Arguments are positional, in the
//! order the variables first appear in the template:
//!
//! ```
//! mod templates {
//!     // The real example uses "templates/"; this crate keeps its doc fixtures here.
//!     typed_handlebars::directory!("doc-templates/");
//! }
//!
//! assert_eq!(
//!     templates::button(42, "Save").render(),
//!     r#"<button id="btn42">Save</button>"#
//! );
//! ```
//!
//! # The builder
//!
//! Every template also gets a builder. It is optional, but it names each variable, so argument
//! order stops mattering and a renamed template variable becomes a compile error rather than a
//! silently transposed argument:
//!
//! ```
//! mod templates {
//!     typed_handlebars::directory!("doc-templates/");
//! }
//!
//! let html = templates::button::Builder::new()
//!     .btn_name("Save")
//!     .btn_id(42)
//!     .render();
//! assert_eq!(html, r#"<button id="btn42">Save</button>"#);
//! ```
//!
//! You set only what you have. Anything left out renders as Handlebars renders an undefined
//! variable — as nothing, an empty list, or a false condition:
//!
//! ```
//! # mod templates { typed_handlebars::directory!("doc-templates/"); }
//! assert_eq!(
//!     templates::button::Builder::new().btn_id(42).render(),
//!     r#"<button id="btn42"></button>"#
//! );
//! ```
//!
//! # Rendering
//!
//! A template value implements [`Display`](core::fmt::Display), so it can be nested inside another
//! template or written straight into a buffer you already have — no intermediate `String` per
//! level:
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
//! # The supported subset
//!
//! Anything outside the supported subset is a compile error that names the construct — never a
//! silent difference in output, and never a Rust type error to decode. The README has the full
//! table of what works, what is not implemented yet, and what is deliberately out of scope
//! (helpers and runtime template loading).
//!
//! # Everything else in this crate
//!
//! [`Empty`], [`escape`], [`Escaped`], [`Truthy`], [`Set`] and [`IsSet`] are the runtime support
//! that generated code calls into. They are public because the generated code names them, not
//! because you need to: there is nothing here for you to implement.

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

#[cfg(test)]
mod tests {

    #[test]
    fn basic_usage() {
        mod template {
            crate::str!("test", r#"<p>{{firstname}} {{lastname}}</p>"#);
        }
        assert_eq!(
            template::test("King", "Tubby").render(),
            "<p>King Tubby</p>"
        );
    }

    /// `{{ }}` escapes and `{{{ }}}` does not, as Handlebars specifies. These used to emit
    /// identical code, so `{{ }}` passed markup straight through.
    #[test]
    fn double_braces_escape_and_triple_braces_do_not() {
        mod template {
            crate::str!("test", r#"<p>{{ two }}|{{{ three }}}</p>"#);
        }
        assert_eq!(
            template::test("a&b<c>", "a&b<c>").render(),
            "<p>a&amp;b&lt;c&gt;|a&b<c></p>"
        );
    }

    /// The same characters handlebars.js escapes, so output matches it exactly. Mirrored in
    /// `reference-ts`.
    #[test]
    fn escaping_covers_the_handlebars_character_set() {
        mod template {
            crate::str!("test", r#"{{ value }}"#);
        }
        assert_eq!(
            template::test(r#"& < > " ' ` ="#).render(),
            "&amp; &lt; &gt; &quot; &#x27; &#x60; &#x3D;"
        );
        // Text with nothing to escape passes through untouched.
        assert_eq!(template::test("plain text 123").render(), "plain text 123");
        // Multi-byte characters are not disturbed.
        assert_eq!(template::test("héllo → <b>").render(), "héllo → &lt;b&gt;");
    }

    /// Escaping is about how a value is written, so it applies wherever a value is written.
    #[test]
    fn escaping_applies_inside_blocks_and_records() {
        mod list {
            crate::str!("test", r#"{{#each rows}}<li>{{name}}</li>{{/each}}"#);
        }
        assert_eq!(
            list::test(vec![list::test::RowsItem::new("Tom & Jerry")]).render(),
            "<li>Tom &amp; Jerry</li>"
        );

        mod record {
            crate::str!("test", r#"{{person.name}}"#);
        }
        assert_eq!(
            record::test(record::test::Person::new("<script>")).render(),
            "&lt;script&gt;"
        );
    }

    /// A template is `Display`, so a nested one goes straight into the parent's buffer instead of
    /// being rendered to a `String` first and copied in.
    #[test]
    fn a_template_can_be_nested_without_an_intermediate_string() {
        mod row {
            crate::str!("test", r#"<li>{{name}}</li>"#);
        }
        mod page {
            crate::str!("test", r#"<ul>{{{ rows }}}</ul>"#);
        }
        // No `.render()` on the inner template — it is passed as a value.
        assert_eq!(
            page::test(row::test("King")).render(),
            "<ul><li>King</li></ul>"
        );
        // …and `Display` means the usual conversions work too.
        assert_eq!(row::test("King").to_string(), "<li>King</li>");
        assert_eq!(format!("{}", row::test("Tubby")), "<li>Tubby</li>");
    }

    /// `render_to` writes into a caller-supplied sink, so a response buffer never needs a
    /// throwaway `String`.
    #[test]
    fn render_to_writes_into_any_sink() {
        use core::fmt::Write;

        mod template {
            crate::str!("test", r#"<p>{{name}}</p>"#);
        }

        let mut buffer = String::from("<body>");
        template::test("King").render_to(&mut buffer).unwrap();
        buffer.push_str("</body>");
        assert_eq!(buffer, "<body><p>King</p></body>");

        /// A sink that is not a `String`, to show the bound is real.
        struct Counter(usize);
        impl Write for Counter {
            fn write_str(&mut self, text: &str) -> core::fmt::Result {
                self.0 += text.len();
                Ok(())
            }
        }
        let mut counter = Counter(0);
        template::test("King").render_to(&mut counter).unwrap();
        assert_eq!(counter.0, "<p>King</p>".len());
    }

    /// The person writing a template picks the variable names and has no reason to know what Rust
    /// reserves. Every one of these used to be a proc-macro panic or a raw Rust syntax error.
    #[test]
    fn variable_names_may_be_rust_keywords() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"[{{ type }}][{{ match }}][{{ fn }}][{{ self }}][{{ crate }}][{{ loop }}]"#
            );
        }
        assert_eq!(
            template::test("a", "b", "c", "d", "e", "f").render(),
            "[a][b][c][d][e][f]"
        );
        // The builder renames them the same way, so autocomplete still finds them.
        assert_eq!(
            template::test::Builder::new()
                .type_("a")
                .self_("d")
                .render(),
            "[a][][][d][][]"
        );
    }

    /// handlebars.js reads `{{2nd}}` as a variable reference, so this does too. Mirrored in
    /// `reference-ts`.
    #[test]
    fn variable_names_may_start_with_a_digit() {
        mod template {
            crate::str!("test", r#"[{{ 2nd }}][{{ 42 }}]"#);
        }
        assert_eq!(
            template::test("silver", "answer").render(),
            "[silver][answer]"
        );
    }

    /// Pre-rendered markup goes in `{{{ }}}`, which is how Handlebars does it too.
    #[test]
    fn a_nested_template_can_be_passed_through_triple_braces() {
        mod row {
            crate::str!("test", r#"<li>{{name}}</li>"#);
        }
        mod page {
            crate::str!("test", r#"<ul>{{{ rows }}}</ul>"#);
        }
        let rows = row::test("King").render();
        assert_eq!(page::test(rows).render(), "<ul><li>King</li></ul>");
    }

    #[test]
    fn path_expressions() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{person.firstname}} {{person.lastname}}"#
            );
        }
        assert_eq!(
            template::test(template::test::Person::new("King", "Tubby")).render(),
            "King Tubby"
        );
    }

    #[test]
    fn if_helper() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#if has_author}}<h1>{{first_name}} {{last_name}}</h1>{{/if}}</div>"#
            );
        }
        assert_eq!(
            template::test(true, "King", "Tubby").render(),
            //language=html
            "<div><h1>King Tubby</h1></div>"
        );
        assert_eq!(
            template::test(false, "King", "Tubby").render(),
            //language=html
            "<div></div>"
        );
    }

    /// The idiom that could not be expressed at all before: test a variable and then print it.
    /// Testing one no longer forces it to be a `bool`.
    #[test]
    fn a_variable_can_be_tested_and_printed() {
        mod template {
            crate::str!("test", r#"[{{#if title}}{{title}}{{/if}}]"#);
        }
        assert_eq!(template::test(String::from("Dub")).render(), "[Dub]");
        assert_eq!(template::test(String::new()).render(), "[]");
        assert_eq!(template::test("Dub").render(), "[Dub]");
        assert_eq!(template::test("").render(), "[]");
        assert_eq!(template::test(7).render(), "[7]");
        assert_eq!(template::test(0).render(), "[]");
        assert_eq!(template::test(true).render(), "[true]");
        assert_eq!(template::test(false).render(), "[]");
    }

    /// What counts as falsy, following handlebars.js. Mirrored in `reference-ts`.
    #[test]
    fn falsiness_follows_handlebars() {
        mod template {
            crate::str!("test", r#"[{{#if value}}yes{{/if}}]"#);
        }
        // Absent and false.
        assert_eq!(template::test::Builder::new().render(), "[]");
        assert_eq!(template::test(false).render(), "[]");
        assert_eq!(template::test(true).render(), "[yes]");
        // Empty string and zero.
        assert_eq!(template::test("").render(), "[]");
        assert_eq!(template::test("x").render(), "[yes]");
        assert_eq!(template::test(0).render(), "[]");
        assert_eq!(template::test(-1).render(), "[yes]");
        // Option is present or not, whatever it wraps.
        assert_eq!(template::test(None::<&str>).render(), "[]");
        assert_eq!(template::test(Some("")).render(), "[yes]");
        // A list is falsy when it has no items.
        assert_eq!(template::test(Vec::<u8>::new()).render(), "[]");
        assert_eq!(template::test(vec![1]).render(), "[yes]");
        // …including through a reference.
        let rows = vec![1];
        assert_eq!(template::test(&rows).render(), "[yes]");
    }

    /// A list can be tested and then walked — the same variable serving both.
    #[test]
    fn a_list_can_be_tested_and_iterated() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#if rows}}<ul>{{#each rows}}<li>{{name}}</li>{{/each}}</ul>{{/if}}"#
            );
        }
        assert_eq!(
            template::test(vec![template::test::RowsItem::new("King")]).render(),
            //language=html
            "<ul><li>King</li></ul>"
        );
        assert_eq!(
            template::test::Builder::new().render(),
            "",
            "no rows, no list"
        );
    }

    /// `{{@index}}` inside `{{#each}}`. Mirrored in `reference-ts`.
    #[test]
    fn each_index() {
        mod template {
            crate::str!("test", r#"{{#each rows}}{{@index}}:{{name}} {{/each}}"#);
        }
        assert_eq!(
            template::test(vec![
                template::test::RowsItem::new("a"),
                template::test::RowsItem::new("b"),
            ])
            .render(),
            "0:a 1:b "
        );
    }

    /// `{{else}}` inside `{{#each}}` covers the empty list. Mirrored in `reference-ts`.
    #[test]
    fn each_else() {
        mod template {
            crate::str!("test", r#"{{#each rows}}{{name}}{{else}}none{{/each}}"#);
        }
        assert_eq!(
            template::test(vec![template::test::RowsItem::new("a")]).render(),
            "a"
        );
        assert_eq!(template::test::Builder::new().render(), "none");
    }

    /// `{{else}}` inside `{{#unless}}`. Mirrored in `reference-ts`.
    #[test]
    fn unless_else() {
        mod template {
            crate::str!("test", r#"{{#unless a}}no{{else}}yes{{/unless}}"#);
        }
        assert_eq!(template::test(false).render(), "no");
        assert_eq!(template::test(true).render(), "yes");
    }

    #[test]
    fn unless_helper() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#unless has_author}}<h1>Unknown</h1>{{/unless}}</div>"#
            );
        }
        assert_eq!(
            template::test(false).render(),
            //language=html
            "<div><h1>Unknown</h1></div>"
        );
        assert_eq!(
            template::test(true).render(),
            //language=html
            "<div></div>"
        );
    }

    #[test]
    fn if_else_helper() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#if has_author}}<h1>{{first_name}}</h1>{{else}}<h1>Unknown</h1>{{/if}}</div>"#
            );
        }
        assert_eq!(
            template::test(true, "King").render(),
            //language=html
            r#"<div><h1>King</h1></div>"#
        );
        assert_eq!(
            template::test(false, "King").render(),
            //language=html
            r#"<div><h1>Unknown</h1></div>"#
        );
    }

    #[test]
    fn with_helper() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#with author}}<h1>{{first_name}} {{last_name}}</h1>{{/with}}</div>"#
            );
        }
        assert_eq!(
            template::test(template::test::Author::new("King", "Tubby")).render(),
            //language=html
            "<div><h1>King Tubby</h1></div>"
        );
    }

    /// Pins the one place this parts company with handlebars.js, so the divergence is visible
    /// rather than folklore: handlebars.js skips a `{{#with}}` block whose subject is undefined,
    /// but an unset record here is a record of empties, so the block still renders.
    #[test]
    fn with_renders_even_when_the_record_was_never_set() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#with author}}<h1>{{first_name}}</h1>{{/with}}</div>"#
            );
        }
        assert_eq!(
            template::test::Builder::new().render(),
            //language=html
            "<div><h1></h1></div>",
            "handlebars.js would render <div></div> here"
        );
    }

    #[test]
    fn for_helper() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#each authors}}<p>Hello {{first_name}}</p>{{/each}}</div>"#
            );
        }
        assert_eq!(
            template::test(vec![template::test::AuthorsItem::new("King")]).render(),
            //language=html
            "<div><p>Hello King</p></div>"
        );
    }

    /// The template says `rows` is a list whose items have a `name`, so the macro generates the
    /// item type. Nothing here declares a type or implements a trait.
    #[test]
    fn each_generates_the_item_type() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<ul>{{#each rows}}<li>{{name}} {{email}}</li>{{/each}}</ul>"#
            );
        }
        assert_eq!(
            template::test(vec![
                template::test::RowsItem::new("King", "king@example.com"),
                template::test::RowsItem::new("Tubby", "tubby@example.com"),
            ])
            .render(),
            //language=html
            "<ul><li>King king@example.com</li><li>Tubby tubby@example.com</li></ul>"
        );
    }

    /// An `{{#each}}` whose body only writes `{{this}}` iterates values, not records, so no item
    /// struct is generated.
    #[test]
    fn each_over_plain_values_needs_no_item_type() {
        mod template {
            crate::str!("test", r#"{{#each tags}}[{{this}}]{{/each}}"#);
        }
        assert_eq!(template::test(vec!["a", "b"]).render(), "[a][b]");
        assert_eq!(template::test([1, 2, 3]).render(), "[1][2][3]");
    }

    #[test]
    fn nested_each_generates_nested_item_types() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each rows}}<tr>{{#each cells}}<td>{{value}}</td>{{/each}}</tr>{{/each}}"#
            );
        }
        let rows = vec![
            template::test::RowsItem::new(vec![
                template::test::RowsItemCellsItem::new(1),
                template::test::RowsItemCellsItem::new(2),
            ]),
            template::test::RowsItem::new(vec![template::test::RowsItemCellsItem::new(3)]),
        ];
        assert_eq!(
            template::test(rows).render(),
            //language=html
            "<tr><td>1</td><td>2</td></tr><tr><td>3</td></tr>"
        );
    }

    /// Without a declared type, `{{ person.name }}` generates the record it implies.
    #[test]
    fn dotted_paths_generate_a_record_type() {
        mod template {
            crate::str!("test", r#"{{person.firstname}} {{person.lastname}}"#);
        }
        assert_eq!(
            template::test(template::test::Person::new("King", "Tubby")).render(),
            "King Tubby"
        );
    }

    #[test]
    fn each_can_reach_the_enclosing_scope() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each rows}}<li>{{name}} of {{../company}}</li>{{/each}}"#
            );
        }
        assert_eq!(
            template::test(vec![template::test::RowsItem::new("King")], "Studio One").render(),
            //language=html
            "<li>King of Studio One</li>"
        );
    }

    #[test]
    fn each_accepts_a_named_item() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each rows as |row|}}<li>{{row.name}}</li>{{/each}}"#
            );
        }
        assert_eq!(
            template::test(vec![template::test::RowsItem::new("King")]).render(),
            //language=html
            "<li>King</li>"
        );
    }

    /// Handlebars renders an undefined variable as nothing, and the builder does the same: you set
    /// what you have.
    #[test]
    fn the_builder_leaves_unset_variables_empty() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<h1>{{title}}</h1><p>{{body}}</p>"#
            );
        }
        assert_eq!(
            template::test::Builder::new().render(),
            "<h1></h1><p></p>",
            "nothing set at all"
        );
        assert_eq!(
            template::test::Builder::new().title("Dub").render(),
            "<h1>Dub</h1><p></p>",
            "only one of the two set"
        );
    }

    /// An unset list has no items, and an unset condition is false — again matching Handlebars.
    #[test]
    fn unset_lists_and_conditions_are_empty() {
        mod list {
            crate::str!("test", r#"[{{#each rows}}<li>{{name}}</li>{{/each}}]"#);
        }
        assert_eq!(list::test::Builder::new().render(), "[]");

        mod conditional {
            crate::str!("test", r#"[{{#if shown}}yes{{/if}}]"#);
        }
        assert_eq!(conditional::test::Builder::new().render(), "[]");
        assert_eq!(
            conditional::test::Builder::new().shown(true).render(),
            "[yes]"
        );
    }

    /// A record left out renders as a record whose own fields are all empty.
    #[test]
    fn an_unset_record_is_empty_all_the_way_down() {
        mod template {
            crate::str!("test", r#"[{{person.first}}|{{person.last}}]"#);
        }
        assert_eq!(template::test::Builder::new().render(), "[|]");
        assert_eq!(
            template::test::Builder::new()
                .person(template::test::PersonBuilder::new().first("King").build())
                .render(),
            "[King|]",
            "a record can itself be partly set"
        );
    }

    /// The wiring API: named setters, so nothing depends on argument order and every name comes
    /// from autocomplete rather than being retyped from the template.
    #[test]
    fn the_builder_is_order_independent() {
        mod template {
            crate::str!("test", r#"<p>{{firstname}} {{lastname}}</p>"#);
        }
        assert_eq!(
            template::test::Builder::new()
                .firstname("King")
                .lastname("Tubby")
                .render(),
            "<p>King Tubby</p>"
        );
        // …and the other way round.
        assert_eq!(
            template::test::Builder::new()
                .lastname("Tubby")
                .firstname("King")
                .render(),
            "<p>King Tubby</p>"
        );
    }

    /// Generated item types get builders too, so a nested shape is wired up the same way.
    #[test]
    fn the_builder_wires_up_lists() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<h1>{{title}}</h1>{{#each rows}}<li>{{name}} {{email}}</li>{{/each}}"#
            );
        }
        let rows = vec![
            template::test::RowsItemBuilder::new()
                .name("King")
                .email("king@example.com")
                .build(),
            template::test::RowsItemBuilder::new()
                .email("tubby@example.com")
                .name("Tubby")
                .build(),
        ];
        let expected = "<h1>Dub</h1><li>King king@example.com</li><li>Tubby tubby@example.com</li>";

        assert_eq!(
            template::test::Builder::new()
                .title("Dub")
                .rows(&rows)
                .render(),
            expected
        );

        // `build` hands back the template value, which can be rendered more than once.
        let page = template::test::Builder::new()
            .rows(rows)
            .title("Dub")
            .build();
        assert_eq!(page.render(), expected);
        assert_eq!(page.render(), expected);
    }

    /// Rendering borrows, so a caller can pass a list they still own — no clone, no giving up the
    /// data.
    #[test]
    fn each_accepts_borrowed_lists() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each rows}}<li>{{name}}</li>{{/each}}"#
            );
        }
        let rows = vec![
            template::test::RowsItem::new("King"),
            template::test::RowsItem::new("Tubby"),
        ];
        let expected = "<li>King</li><li>Tubby</li>";

        assert_eq!(template::test(&rows).render(), expected);
        assert_eq!(template::test(rows.as_slice()).render(), expected);

        // The caller still owns it, and can hand it over afterwards if they want to.
        assert_eq!(rows.len(), 2);
        assert_eq!(template::test(rows).render(), expected);

        let array = [template::test::RowsItem::new("King")];
        assert_eq!(template::test(&array).render(), "<li>King</li>");
        assert_eq!(template::test(array).render(), "<li>King</li>");
    }

    /// Rendering borrows rather than consumes, so a template may walk the same list twice.
    #[test]
    fn the_same_list_can_be_iterated_twice() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each rows}}{{name}}{{/each}}|{{#each rows}}{{name}}{{/each}}"#
            );
        }
        let page = template::test(vec![
            template::test::RowsItem::new("a"),
            template::test::RowsItem::new("b"),
        ]);
        assert_eq!(page.render(), "ab|ab");
        // …and the value is still usable afterwards.
        assert_eq!(page.render(), "ab|ab");
    }

    #[test]
    fn test_comment() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"Note: {{! This is a comment }} and {{!-- {{so is this}} --}}\\{{{{}}"#,
            );
        }
        assert_eq!(template::test().render(), "Note:  and \\{{");
    }

    #[test]
    fn test_trimming() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"  {{~#if some ~}}   Hello{{~/if~}}"#,
            );
        }
        assert_eq!(template::test(true).render(), "Hello");
    }

    #[test]
    fn it_works() {
        mod template {
            crate::str!("test", "Hello {{{name}}}!");
        }
        assert_eq!(template::test("King").render(), "Hello King!");
    }

    #[test]
    fn test_escaped() {
        mod template {
            crate::str!(
                "test",
                "{{{{skip}}}}wang doodle {{{{/dandy}}}}{{{{/skip}}}}"
            );
        }
        assert_eq!(template::test().render(), "wang doodle {{{{/dandy}}}}");
    }

    // #[test]
    // fn test_nesting() {
    //     let rust = compile("{{#if some}}{{#each some}}Hello {{this}}{{/each}}{{/if}}");
    //     assert_eq!(
    //         rust,
    //         "if self.some.as_bool(){for this_2 in self.some{write!(f, \"Hello {}\", this_2.as_display_html())?;}}"
    //     );
    // }
    //
    // #[test]
    // fn test_as() {
    //     let rust = compile(
    //         "{{#if some}}{{#each some as thing}}Hello {{thing}} {{thing.name}}{{/each}}{{/if}}",
    //     );
    //     assert_eq!(
    //         rust,
    //         "if self.some.as_bool(){for thing_2 in self.some{write!(f, \"Hello {} {}\", thing_2.as_display_html(), thing_2.name.as_display_html())?;}}"
    //     );
    // }
    //
    // #[test]
    // fn test_scoping() {
    //     let rust = compile(
    //         "{{#with some}}{{#with other}}Hello {{name}} {{../company}} {{/with}}{{/with}}",
    //     );
    //     assert_eq!(
    //         rust,
    //         "{let this_1 = self.some;{let this_2 = this_1.other;write!(f, \"Hello {} {} \", this_2.name.as_display_html(), this_1.company.as_display_html())?;}}"
    //     );
    // }
    //
    // #[test]
    // fn test_indexer() {
    //     let rust = compile(
    //         "{{#each things}}Hello{{{@index}}}{{#each things}}{{{lookup other @../index}}}{{{@index}}}{{/each}}{{/each}}",
    //     );
    //     assert_eq!(
    //         rust,
    //         "let mut i_1 = 0;for this_1 in self.things{write!(f, \"Hello{}\", i_1.as_display())?;let mut i_2 = 0;for this_2 in this_1.things{write!(f, \"{}{}\", this_2.other[i_1].as_display(), i_2.as_display())?;i_2+=1;}i_1+=1;}"
    //     );
    // }
    //
    // #[test]
    // fn test_map() {
    //     let rust = compile(
    //         "{{#each things}}Hello{{{@key}}}{{#each @value}}{{#if_some (try_lookup other @../key)}}{{{this}}}{{/if_some}}{{{@value}}}{{/each}}{{/each}}",
    //     );
    //     assert_eq!(
    //         rust,
    //         "for this_1 in self.things{write!(f, \"Hello{}\", this_1.0.as_display())?;for this_2 in this_1.1{if let Some(this_3) = this_2.other.get(this_1.0){write!(f, \"{}\", this_3.as_display())?;}write!(f, \"{}\", this_2.1.as_display())?;}}"
    //     );
    // }
    //
    //
    // #[test]
    // fn test_subexpression() {
    //     let rust = compile(
    //         "{{#each things}}{{#with (lookup ../other @index) as |other|}}{{{../name}}}: {{{other}}}{{/with}}{{/each}}",
    //     );
    //     assert_eq!(
    //         rust,
    //         "let mut i_1 = 0;for this_1 in self.things{{let other_2 = self.other[i_1];write!(f, \"{}: {}\", this_1.name.as_display(), other_2.as_display())?;}i_1+=1;}"
    //     );
    // }
    //
    // #[test]
    // fn test_selfless() {
    //     let rust = Compiler::new(Options{
    //         root_var_name: None,
    //         write_var_name: "f",
    //         variable_types: Default::default(),
    //     }, make_map()).compile("{{#each things}}{{#with (lookup ../other @index) as |other|}}{{{../name}}}: {{{other}}}{{/with}}{{/each}}").unwrap();
    //     assert_eq!(
    //         rust.uses("rusty_handlebars").to_string(),
    //         "use rusty_handlebars::AsDisplay"
    //     );
    //     assert_eq!(
    //         rust.code,
    //         "let mut i_1 = 0;for this_1 in things{{let other_2 = other[i_1];write!(f, \"{}: {}\", this_1.name.as_display(), other_2.as_display())?;}i_1+=1;}"
    //     );
    // }
    //
    // #[test]
    // fn javascript() {
    //     let rust = Compiler::new(opts(), make_map()).compile("<script>if (location.href.contains(\"localhost\")){ console.log(\"\\{{{{}}}}\") }</script>").unwrap();
    //     assert_eq!(rust.uses("rusty_handlebars").to_string(), "");
    //     assert_eq!(
    //         rust.code,
    //         "write!(f, \"<script>if (location.href.contains(\\\"localhost\\\")){{ console.log(\\\"{{{{}}}}\\\") }}</script>\")?;"
    //     );
    // }
}
