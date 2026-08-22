//! `{{ t "Save" }}` — a call to a method on the frame named by `register_helper!`.
//! Mirrored in `reference-ts`, which checks the same output against real handlebars.js with the
//! equivalent `registerHelper`.
//!
//! Handlebars gives a template two things: the data, and a *data frame* of ambient state passed at
//! render time. `Vars` is the data; the frame is what a helper resolves on.
//!
//! An integration test, so the macro is reached the way a consumer reaches it.

/// A frame: what a request or a session would carry, standing in for the real thing.
pub struct Ctx {
    greeting: &'static str,
}

impl Ctx {
    pub fn t(&self, key: &str) -> String {
        format!("{} {}", self.greeting, key)
    }

    /// Returns a borrow rather than a `String`, since a translation table usually can.
    pub fn upper(&self, key: &str) -> &'static str {
        match key {
            "save" => "SAVE",
            _ => "?",
        }
    }

    pub fn join(&self, left: &str, right: &str) -> String {
        format!("{}/{}", left, right)
    }

    pub fn shout(&self, value: &str) -> String {
        format!("{}!", value)
    }

    /// Markup, so escaping has something to bite on.
    pub fn tag(&self, name: &str) -> String {
        format!("<{}>", name)
    }
}

fn ctx() -> Ctx {
    Ctx { greeting: "Hello" }
}

/// The plain case: a literal key, and a frame passed to `render` beside the data.
#[test]
fn a_helper_is_a_method_on_the_frame() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("test", r#"<p>{{ t "world" }}</p>"#);
    }
    assert_eq!(template::test::Vars.render(&ctx()), "<p>Hello world</p>");
}

/// Handlebars accepts both spellings of a string literal, and a designer has no reason to know
/// that one of them was easier to lex.
#[test]
fn a_key_may_be_quoted_either_way() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("test", r#"{{ t "world" }}|{{ t 'world' }}"#);
    }
    assert_eq!(
        template::test::Vars.render(&ctx()),
        "Hello world|Hello world"
    );
}

/// A number is a literal where it is an argument, handed over as the text the template spelled.
/// `{{ 42 }}` on its own stays a path — see `generated_types::variable_names_may_start_with_a_digit`.
#[test]
fn a_number_argument_arrives_as_its_own_text() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("test", r#"{{ shout 123 }}|{{ shout -1.5 }}"#);
    }
    assert_eq!(template::test::Vars.render(&ctx()), "123!|-1.5!");
}

/// Anything that is not a literal is data, so it becomes a field — and reaches the helper as the
/// text it would have been written as.
#[test]
fn a_variable_argument_becomes_a_field() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("test", r#"{{ shout total }}"#);
    }
    assert_eq!(template::test::Vars { total: 4200 }.render(&ctx()), "4200!");
    // Any type the template could have written out, not just strings.
    assert_eq!(template::test::Vars { total: "x" }.render(&ctx()), "x!");
}

/// A literal argument names no data, so it adds no field. `Vars` stays exactly the variables.
#[test]
fn a_literal_argument_adds_no_field() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("test", r#"{{ t "world" }}{{ name }}"#);
    }
    // A second field would make this a `missing field` error rather than a passing test.
    assert_eq!(
        template::test::Vars { name: "King" }.render(&ctx()),
        "Hello worldKing"
    );
}

/// Arity is the method's business, so more than one argument needs nothing extra.
#[test]
fn a_helper_takes_as_many_arguments_as_its_method() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("test", r#"{{ join left "USD" }}"#);
    }
    assert_eq!(template::test::Vars { left: 42 }.render(&ctx()), "42/USD");
}

/// The return goes through `Render` like any other written value, so it need not be a `String`.
#[test]
fn a_helper_may_return_a_borrow() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("test", r#"{{ upper "save" }}"#);
    }
    assert_eq!(template::test::Vars.render(&ctx()), "SAVE");
}

/// A helper's result is written like any other value, so `{{ }}` escapes it and `{{{ }}}` does
/// not. handlebars.js does the same unless the helper returns a `SafeString`.
#[test]
fn a_helper_result_is_escaped_like_any_other_value() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("test", r#"{{ tag "b" }}|{{{ tag "b" }}}"#);
    }
    assert_eq!(template::test::Vars.render(&ctx()), "&lt;b&gt;|<b>");
}

/// Inside a loop the frame is unchanged — it is ambient, not part of the data — while an argument
/// resolves in the scope it sits in.
#[test]
fn a_helper_works_inside_a_loop() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!(
            "test",
            r#"{{#each rows}}[{{ shout name }}{{ t "x" }}]{{/each}}"#
        );
    }
    let rows = vec![
        template::test::RowsItem { name: "a" },
        template::test::RowsItem { name: "b" },
    ];
    assert_eq!(
        template::test::Vars { rows }.render(&ctx()),
        "[a!Hello x][b!Hello x]"
    );
}

/// The builder reaches the same render, so it takes the frame too.
#[test]
fn the_builder_takes_the_frame_as_well() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("test", r#"{{ t "world" }}{{ name }}"#);
    }
    assert_eq!(template::test::builder().render(&ctx()), "Hello world");
}

/// Only templates that call a helper take the frame. One that calls none is untouched, even in a
/// module where `register_helper!` was used.
#[test]
fn a_template_without_a_helper_still_renders_on_its_own() {
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("plain", r#"<p>{{ name }}</p>"#);
        typed_handlebars::str!("helped", r#"<p>{{ t "world" }}</p>"#);
    }
    assert_eq!(
        template::plain::Vars { name: "King" }.render(),
        "<p>King</p>"
    );
    assert_eq!(template::helped::Vars.render(&ctx()), "<p>Hello world</p>");
}

/// `render_to` is the one that actually writes, and it takes the frame in the same place.
#[test]
fn render_to_takes_the_frame_too() {
    use core::fmt::Write as _;
    mod template {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("test", r#"{{ t "world" }}"#);
    }
    let mut out = String::new();
    write!(&mut out, "[").unwrap();
    template::test::Vars.render_to(&mut out, &ctx()).unwrap();
    write!(&mut out, "]").unwrap();
    assert_eq!(out, "[Hello world]");
}
