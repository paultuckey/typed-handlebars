//! Template syntax that produces no data of its own: comments, whitespace control,
//! and the literal forms.
//!
//! An integration test, so the macro is reached the way a consumer reaches it — through
//! `typed_handlebars::str!` rather than `crate::str!`. That also covers the path resolution these
//! tests used to sit on the wrong side of: generated code names the runtime crate absolutely, and
//! from here that name has to resolve to a dependency rather than to the crate under test.

#[test]
fn test_comment() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"Note: {{! This is a comment }} and {{!-- {{so is this}} --}}\\{{{{}}"#,
        );
    }
    assert_eq!(template::test().render(), "Note:  and \\{{");
}

/// A comment can close with a `~` inside the token — `{{! … ~}}` and `{{!-- … --~}}` — which
/// trims the whitespace after it. The long form used to report itself as unclosed, because the
/// `~` sits between the `--` and the `}}` rather than before them. Mirrored in `reference-ts`.
#[test]
fn a_comment_can_trim_the_whitespace_after_it() {
    mod long {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "x{{!-- c --~}}   y"
        );
    }
    assert_eq!(long::test().render(), "xy");

    mod short {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "x{{! c ~}}   y"
        );
    }
    assert_eq!(short::test().render(), "xy");

    // Both ends at once, and the trim reaches across newlines.
    mod both_ends {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "x   {{~!-- c --~}}\n\n   y"
        );
    }
    assert_eq!(both_ends::test().render(), "xy");

    // Without the `~` the whitespace stays, which is what makes the test above mean something.
    mod untrimmed {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "x{{!-- c --}}   y"
        );
    }
    assert_eq!(untrimmed::test().render(), "x   y");
}

/// Whichever close comes first wins, so a `--~}}` after the comment has already ended is just
/// text. Checked against handlebars.js, which does the same.
#[test]
fn a_comment_ends_at_its_first_close() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "x{{!-- a --}} b --~}}   y"
        );
    }
    assert_eq!(template::test().render(), "x b --~}}   y");
}

/// A comment is the one expression with no name, so an empty one is legal rather than
/// "empty expression". `{{}}` is still an error — handlebars.js rejects that too.
#[test]
fn a_comment_may_be_empty() {
    mod short {
        typed_handlebars::str!(
            "test", //language=handlebars
            "x{{!}}y"
        );
    }
    assert_eq!(short::test().render(), "xy");

    mod long {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "x{{!----}}y"
        );
    }
    assert_eq!(long::test().render(), "xy");

    mod trimming {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "x{{!~}}   y"
        );
    }
    assert_eq!(trimming::test().render(), "xy");
}

/// The long form exists so a comment can contain `}}`, and a bare `~}}` inside one is text
/// rather than a close — only `--~}}` ends it.
#[test]
fn a_long_comment_swallows_braces_and_stray_tildes() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "x{{!-- {{a}} and ~}} and -- --}}y"
        );
    }
    assert_eq!(template::test().render(), "xy");
}

#[test]
fn test_trimming() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"  {{~#if some ~}}   Hello{{~/if~}}"#,
        );
    }
    assert_eq!(template::test(true).render(), "Hello");
}

#[test]
fn test_escaped() {
    mod template {
        typed_handlebars::str!(
            "test",
            "{{{{skip}}}}wang doodle {{{{/dandy}}}}{{{{/skip}}}}"
        );
    }
    assert_eq!(template::test().render(), "wang doodle {{{{/dandy}}}}");
}
