//! How a template turns into output: the constructor, `render`, `render_to`, and
//! `Display` for nesting one template inside another.
//!
//! An integration test, so the macro is reached the way a consumer reaches it — through
//! `typed_handlebars::str!` rather than `crate::str!`. That also covers the path resolution these
//! tests used to sit on the wrong side of: generated code names the runtime crate absolutely, and
//! from here that name has to resolve to a dependency rather than to the crate under test.

#[test]
fn basic_usage() {
    mod template {
        typed_handlebars::str!("test", r#"<p>{{firstname}} {{lastname}}</p>"#);
    }
    assert_eq!(
        template::test("King", "Tubby").render(),
        "<p>King Tubby</p>"
    );
}

#[test]
fn path_expressions() {
    mod template {
        typed_handlebars::str!(
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

/// A template is `Display`, so a nested one goes straight into the parent's buffer instead of
/// being rendered to a `String` first and copied in.
#[test]
fn a_template_can_be_nested_without_an_intermediate_string() {
    mod row {
        typed_handlebars::str!("test", r#"<li>{{name}}</li>"#);
    }
    mod page {
        typed_handlebars::str!("test", r#"<ul>{{{ rows }}}</ul>"#);
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
        typed_handlebars::str!("test", r#"<p>{{name}}</p>"#);
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

/// Pre-rendered markup goes in `{{{ }}}`, which is how Handlebars does it too.
#[test]
fn a_nested_template_can_be_passed_through_triple_braces() {
    mod row {
        typed_handlebars::str!("test", r#"<li>{{name}}</li>"#);
    }
    mod page {
        typed_handlebars::str!("test", r#"<ul>{{{ rows }}}</ul>"#);
    }
    let rows = row::test("King").render();
    assert_eq!(page::test(rows).render(), "<ul><li>King</li></ul>");
}

#[test]
fn it_works() {
    mod template {
        typed_handlebars::str!("test", "Hello {{{name}}}!");
    }
    assert_eq!(template::test("King").render(), "Hello King!");
}
