//! A tag alone on its line leaves no trace: both its indentation and its trailing newline go, as
//! they do in handlebars.js. Without this a template laid out over several lines gains a blank line
//! after every tag — which is most templates.
//!
//! An integration test, so the macro is reached the way a consumer reaches it — through
//! `typed_handlebars::str!` rather than `crate::str!`. That also covers the path resolution these
//! tests used to sit on the wrong side of: generated code names the runtime crate absolutely, and
//! from here that name has to resolve to a dependency rather than to the crate under test.
//!
//! Every expectation here is mirrored in `reference-ts` against real handlebars.js.

/// The case this exists for: a list laid out over several lines. Every line of the output used to
/// carry a blank line after it.
#[test]
fn a_list_over_several_lines_renders_as_written() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "<ul>\n{{#each rows}}\n  <li>{{n}}</li>\n{{/each}}\n</ul>"
        );
    }
    let rows = vec![
        template::test::RowsItem::new(1),
        template::test::RowsItem::new(2),
    ];
    assert_eq!(
        template::test(rows).render(),
        "<ul>\n  <li>1</li>\n  <li>2</li>\n</ul>"
    );
}

/// The block's own indentation goes; the body keeps its own.
#[test]
fn a_standalone_tag_takes_its_indentation_and_its_newline() {
    mod plain {
        typed_handlebars::str!("test", "a\n{{#if x}}\nB\n{{/if}}\nc");
    }
    assert_eq!(plain::test(true).render(), "a\nB\nc");

    mod indented {
        typed_handlebars::str!("test", "a\n  {{#if x}}\n  B\n  {{/if}}\nc");
    }
    assert_eq!(indented::test(true).render(), "a\n  B\nc");

    mod tabbed {
        typed_handlebars::str!("test", "a\n\t{{#if x}}\nB\n{{/if}}\nb");
    }
    assert_eq!(tabbed::test(true).render(), "a\nB\nb");
}

/// Comments and `{{else}}` stand alone too — every tag that produces no output of its own.
#[test]
fn comments_and_else_stand_alone() {
    mod comment {
        typed_handlebars::str!("test", "a\n{{! hi }}\nc");
    }
    assert_eq!(comment::test().render(), "a\nc");

    mod branches {
        typed_handlebars::str!("test", "a\n{{#if x}}\nB\n{{else}}\nC\n{{/if}}\nd");
    }
    assert_eq!(branches::test(true).render(), "a\nB\nd");
    assert_eq!(branches::test(false).render(), "a\nC\nd");
}

/// An interpolation is **not** standalone: it is there to produce output, so its line is real.
/// This is the line between the two halves of the rule, and handlebars.js draws it the same way.
#[test]
fn an_interpolation_is_not_standalone() {
    mod escaped {
        typed_handlebars::str!("test", "a\n{{n}}\nb");
    }
    assert_eq!(escaped::test("N").render(), "a\nN\nb");

    mod raw {
        typed_handlebars::str!("test", "a\n{{{n}}}\nb");
    }
    assert_eq!(raw::test("N").render(), "a\nN\nb");
}

/// "Alone" means alone. Anything else on the line — text or a second tag — and the whitespace is
/// the author's.
#[test]
fn anything_else_on_the_line_cancels_it() {
    mod text_after {
        typed_handlebars::str!("test", "a\n{{#if x}} z\nB\n{{/if}}\nc");
    }
    assert_eq!(text_after::test(true).render(), "a\n z\nB\nc");

    mod two_tags {
        typed_handlebars::str!("test", "a\n{{#if x}}{{/if}}\nb");
    }
    assert_eq!(two_tags::test(true).render(), "a\n\nb");

    mod tag_space_tag {
        typed_handlebars::str!("test", "a\n{{! c }} {{! d }}\nz");
    }
    assert_eq!(tag_space_tag::test().render(), "a\n \nz");
}

/// The start and the end of a template bound a line as a newline does, so a tag against either
/// edge still stands alone.
#[test]
fn the_edges_of_the_template_bound_a_line() {
    mod at_start {
        typed_handlebars::str!("test", "{{#if x}}\nB\n{{/if}}\nc");
    }
    assert_eq!(at_start::test(true).render(), "B\nc");

    mod indented_start {
        typed_handlebars::str!("test", "  {{#if x}}\nB\n{{/if}}\nc");
    }
    assert_eq!(indented_start::test(true).render(), "B\nc");

    mod at_end {
        typed_handlebars::str!("test", "a\n{{#if x}}\nB\n{{/if}}");
    }
    assert_eq!(at_end::test(true).render(), "a\nB\n");

    // Trailing blanks are part of the tag's line, so they go with it.
    mod blanks_at_end {
        typed_handlebars::str!("test", "a\n{{#if x}}\nB\n{{/if}}   ");
    }
    assert_eq!(blanks_at_end::test(true).render(), "a\nB\n");

    mod nothing_but_a_tag {
        typed_handlebars::str!("test", "{{! c }}");
    }
    assert_eq!(nothing_but_a_tag::test().render(), "");
}

/// A standalone tag consumes the newline that ended its line, which puts the next tag at the start
/// of a line even though no newline sits between them. Without carrying that forward, a run of
/// standalone tags would only strip the first.
#[test]
fn standing_alone_carries_forward_to_the_next_tag() {
    mod indented_after {
        typed_handlebars::str!("test", "a\n{{! c }}\n  {{#if x}}\nB\n{{/if}}\nz");
    }
    assert_eq!(indented_after::test(true).render(), "a\nB\nz");

    mod run_of_them {
        typed_handlebars::str!("test", "a\n{{! c }}\n{{! d }}\nz");
    }
    assert_eq!(run_of_them::test().render(), "a\nz");

    // …but it does not make the *following* line's interpolation standalone.
    mod then_an_interpolation {
        typed_handlebars::str!("test", "a\n{{! c }}\n{{n}}\nz");
    }
    assert_eq!(then_an_interpolation::test("N").render(), "a\nN\nz");
}

/// Exactly one newline goes. A blank line around a tag was put there by the author and stays.
#[test]
fn only_the_tags_own_newline_is_taken() {
    mod template {
        typed_handlebars::str!("test", "a\n\n{{! c }}\n\nb");
    }
    assert_eq!(template::test().render(), "a\n\n\nb");
}

/// Explicit `{{~ … ~}}` trimming still wins where it is asked for, and standalone handling does not
/// interfere with it: `~` trims across newlines, which leaves nothing for this rule to match.
#[test]
fn explicit_trimming_still_works() {
    mod template {
        typed_handlebars::str!("test", "  {{~#if some ~}}   Hello{{~/if~}}");
    }
    assert_eq!(template::test(true).render(), "Hello");
}
