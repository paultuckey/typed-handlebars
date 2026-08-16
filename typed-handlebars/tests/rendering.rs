//! How a template turns into output: writing `Vars`, `render`, `render_to`, and nesting one
//! template's output inside another through `{{{ }}}`.
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
        template::test::Vars {
            firstname: "King",
            lastname: "Tubby"
        }
        .render(),
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
        template::test::Vars {
            person: template::test::Person {
                firstname: "King",
                lastname: "Tubby"
            }
        }
        .render(),
        "King Tubby"
    );
}

/// One template's output goes into another through `{{{ }}}`, exactly as handlebars.js renders a
/// fragment and passes the HTML in as a variable. Anything that displays will do, so the inner
/// template is rendered first and its `String` passed along.
#[test]
fn a_rendered_template_can_be_nested_in_another() {
    mod row {
        typed_handlebars::str!("test", r#"<li>{{name}}</li>"#);
    }
    mod page {
        typed_handlebars::str!("test", r#"<ul>{{{ rows }}}</ul>"#);
    }
    assert_eq!(
        page::test::Vars {
            rows: row::test::Vars { name: "King" }.render()
        }
        .render(),
        "<ul><li>King</li></ul>"
    );

    // The point of `{{{ }}}` here: the inner template's markup is not escaped again on the way in.
    mod escapes {
        typed_handlebars::str!("test", r#"<li>{{name}}</li>"#);
    }
    assert_eq!(
        page::test::Vars {
            rows: escapes::test::Vars { name: "A & B" }.render()
        }
        .render(),
        "<ul><li>A &amp; B</li></ul>",
        "the inner template escaped its own value, and the outer left that markup alone"
    );
}

/// Which content goes in can be decided at run time — something a `{{> partial}}` cannot do,
/// because partial names are resolved at compile time.
#[test]
fn nested_content_can_be_chosen_at_run_time() {
    mod page {
        typed_handlebars::str!("test", r#"<main>{{{ content }}}</main>"#);
    }
    mod home {
        typed_handlebars::str!("test", r#"<h1>{{title}}</h1>"#);
    }
    for (logged_in, expected) in [
        (true, "<main><h1>Dub</h1></main>"),
        (false, "<main>please sign in</main>"),
    ] {
        let content = if logged_in {
            home::test::Vars { title: "Dub" }.render()
        } else {
            String::from("please sign in")
        };
        assert_eq!(page::test::Vars { content }.render(), expected);
    }
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
    template::test::Vars { name: "King" }
        .render_to(&mut buffer)
        .unwrap();
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
    template::test::Vars { name: "King" }
        .render_to(&mut counter)
        .unwrap();
    assert_eq!(counter.0, "<p>King</p>".len());
}

#[test]
fn it_works() {
    mod template {
        typed_handlebars::str!("test", "Hello {{{name}}}!");
    }
    assert_eq!(
        template::test::Vars { name: "King" }.render(),
        "Hello King!"
    );
}
