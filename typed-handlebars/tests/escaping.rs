//! `{{ }}` escapes and `{{{ }}}` does not, over the character set handlebars.js uses.
//! Mirrored in `reference-ts`, which checks the same output against real handlebars.js.
//!
//! An integration test, so the macro is reached the way a consumer reaches it — through
//! `typed_handlebars::str!` rather than `crate::str!`. That also covers the path resolution these
//! tests used to sit on the wrong side of: generated code names the runtime crate absolutely, and
//! from here that name has to resolve to a dependency rather than to the crate under test.

/// `{{ }}` escapes and `{{{ }}}` does not, as Handlebars specifies. These used to emit
/// identical code, so `{{ }}` passed markup straight through.
#[test]
fn double_braces_escape_and_triple_braces_do_not() {
    mod template {
        typed_handlebars::str!("test", r#"<p>{{ two }}|{{{ three }}}</p>"#);
    }
    assert_eq!(
        template::test::Vars {
            two: "a&b<c>",
            three: "a&b<c>"
        }
        .render(),
        "<p>a&amp;b&lt;c&gt;|a&b<c></p>"
    );
}

/// The same characters handlebars.js escapes, so output matches it exactly. Mirrored in
/// `reference-ts`.
#[test]
fn escaping_covers_the_handlebars_character_set() {
    mod template {
        typed_handlebars::str!("test", r#"{{ value }}"#);
    }
    assert_eq!(
        template::test::Vars {
            value: r#"& < > " ' ` ="#
        }
        .render(),
        "&amp; &lt; &gt; &quot; &#x27; &#x60; &#x3D;"
    );
    // Text with nothing to escape passes through untouched.
    assert_eq!(
        template::test::Vars {
            value: "plain text 123"
        }
        .render(),
        "plain text 123"
    );
    // Multi-byte characters are not disturbed.
    assert_eq!(
        template::test::Vars {
            value: "héllo → <b>"
        }
        .render(),
        "héllo → &lt;b&gt;"
    );
}

/// Escaping is about how a value is written, so it applies wherever a value is written.
#[test]
fn escaping_applies_inside_blocks_and_records() {
    mod list {
        typed_handlebars::str!("test", r#"{{#each rows}}<li>{{name}}</li>{{/each}}"#);
    }
    assert_eq!(
        list::test::Vars {
            rows: vec![list::test::RowsItem {
                name: "Tom & Jerry"
            }]
        }
        .render(),
        "<li>Tom &amp; Jerry</li>"
    );

    mod record {
        typed_handlebars::str!("test", r#"{{person.name}}"#);
    }
    assert_eq!(
        record::test::Vars {
            person: record::test::Person { name: "<script>" }
        }
        .render(),
        "&lt;script&gt;"
    );
}
