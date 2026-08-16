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
        template::test::RowsItem { n: 1 },
        template::test::RowsItem { n: 2 },
    ];
    assert_eq!(
        template::test::Vars { rows }.render(),
        "<ul>\n  <li>1</li>\n  <li>2</li>\n</ul>"
    );
}

/// The block's own indentation goes; the body keeps its own.
#[test]
fn a_standalone_tag_takes_its_indentation_and_its_newline() {
    mod plain {
        typed_handlebars::str!("test", "a\n{{#if x}}\nB\n{{/if}}\nc");
    }
    assert_eq!(plain::test::Vars { x: true }.render(), "a\nB\nc");

    mod indented {
        typed_handlebars::str!("test", "a\n  {{#if x}}\n  B\n  {{/if}}\nc");
    }
    assert_eq!(indented::test::Vars { x: true }.render(), "a\n  B\nc");

    mod tabbed {
        typed_handlebars::str!("test", "a\n\t{{#if x}}\nB\n{{/if}}\nb");
    }
    assert_eq!(tabbed::test::Vars { x: true }.render(), "a\nB\nb");
}

/// Comments and `{{else}}` stand alone too — every tag that produces no output of its own.
#[test]
fn comments_and_else_stand_alone() {
    mod comment {
        typed_handlebars::str!("test", "a\n{{! hi }}\nc");
    }
    assert_eq!(comment::test::Vars {}.render(), "a\nc");

    mod branches {
        typed_handlebars::str!("test", "a\n{{#if x}}\nB\n{{else}}\nC\n{{/if}}\nd");
    }
    assert_eq!(branches::test::Vars { x: true }.render(), "a\nB\nd");
    assert_eq!(branches::test::Vars { x: false }.render(), "a\nC\nd");
}

/// An interpolation is **not** standalone: it is there to produce output, so its line is real.
/// This is the line between the two halves of the rule, and handlebars.js draws it the same way.
#[test]
fn an_interpolation_is_not_standalone() {
    mod escaped {
        typed_handlebars::str!("test", "a\n{{n}}\nb");
    }
    assert_eq!(escaped::test::Vars { n: "N" }.render(), "a\nN\nb");

    mod raw {
        typed_handlebars::str!("test", "a\n{{{n}}}\nb");
    }
    assert_eq!(raw::test::Vars { n: "N" }.render(), "a\nN\nb");
}

/// "Alone" means alone. Anything else on the line — text or a second tag — and the whitespace is
/// the author's.
#[test]
fn anything_else_on_the_line_cancels_it() {
    mod text_after {
        typed_handlebars::str!("test", "a\n{{#if x}} z\nB\n{{/if}}\nc");
    }
    assert_eq!(text_after::test::Vars { x: true }.render(), "a\n z\nB\nc");

    mod two_tags {
        typed_handlebars::str!("test", "a\n{{#if x}}{{/if}}\nb");
    }
    assert_eq!(two_tags::test::Vars { x: true }.render(), "a\n\nb");

    mod tag_space_tag {
        typed_handlebars::str!("test", "a\n{{! c }} {{! d }}\nz");
    }
    assert_eq!(tag_space_tag::test::Vars {}.render(), "a\n \nz");
}

/// The start and the end of a template bound a line as a newline does, so a tag against either
/// edge still stands alone.
#[test]
fn the_edges_of_the_template_bound_a_line() {
    mod at_start {
        typed_handlebars::str!("test", "{{#if x}}\nB\n{{/if}}\nc");
    }
    assert_eq!(at_start::test::Vars { x: true }.render(), "B\nc");

    mod indented_start {
        typed_handlebars::str!("test", "  {{#if x}}\nB\n{{/if}}\nc");
    }
    assert_eq!(indented_start::test::Vars { x: true }.render(), "B\nc");

    mod at_end {
        typed_handlebars::str!("test", "a\n{{#if x}}\nB\n{{/if}}");
    }
    assert_eq!(at_end::test::Vars { x: true }.render(), "a\nB\n");

    // Trailing blanks are part of the tag's line, so they go with it.
    mod blanks_at_end {
        typed_handlebars::str!("test", "a\n{{#if x}}\nB\n{{/if}}   ");
    }
    assert_eq!(blanks_at_end::test::Vars { x: true }.render(), "a\nB\n");

    mod nothing_but_a_tag {
        typed_handlebars::str!("test", "{{! c }}");
    }
    assert_eq!(nothing_but_a_tag::test::Vars {}.render(), "");
}

/// A standalone tag consumes the newline that ended its line, which puts the next tag at the start
/// of a line even though no newline sits between them. Without carrying that forward, a run of
/// standalone tags would only strip the first.
#[test]
fn standing_alone_carries_forward_to_the_next_tag() {
    mod indented_after {
        typed_handlebars::str!("test", "a\n{{! c }}\n  {{#if x}}\nB\n{{/if}}\nz");
    }
    assert_eq!(indented_after::test::Vars { x: true }.render(), "a\nB\nz");

    mod run_of_them {
        typed_handlebars::str!("test", "a\n{{! c }}\n{{! d }}\nz");
    }
    assert_eq!(run_of_them::test::Vars {}.render(), "a\nz");

    // …but it does not make the *following* line's interpolation standalone.
    mod then_an_interpolation {
        typed_handlebars::str!("test", "a\n{{! c }}\n{{n}}\nz");
    }
    assert_eq!(
        then_an_interpolation::test::Vars { n: "N" }.render(),
        "a\nN\nz"
    );
}

/// Exactly one newline goes. A blank line around a tag was put there by the author and stays.
#[test]
fn only_the_tags_own_newline_is_taken() {
    mod template {
        typed_handlebars::str!("test", "a\n\n{{! c }}\n\nb");
    }
    assert_eq!(template::test::Vars {}.render(), "a\n\n\nb");
}

/// Explicit `{{~ … ~}}` trimming still wins where it is asked for, and standalone handling does not
/// interfere with it: `~` trims across newlines, which leaves nothing for this rule to match.
#[test]
fn explicit_trimming_still_works() {
    mod template {
        typed_handlebars::str!("test", "  {{~#if some ~}}   Hello{{~/if~}}");
    }
    assert_eq!(template::test::Vars { some: true }.render(), "Hello");
}

/// A partial alone on its line is standalone too, and its indentation is not dropped but
/// **applied to every line it emits**. Partials are spliced before parsing, so this half of the
/// rule lives in the assembler rather than in the expression parser.
///
/// The fixtures are in `tests/standalone-templates/`; each expectation here was read off
/// handlebars.js first.
mod partials {
    typed_handlebars::directory!("tests/standalone-templates/");

    /// Every line of the partial gets the indent, not just the first, and the tag's own line goes.
    #[test]
    fn the_indent_reaches_every_line() {
        assert_eq!(
            indented::Vars.render(),
            "start\n    <a>\n    <b>\n    <c>end"
        );
    }

    /// A partial whose text ends in a newline leaves no dangling indent after it — the indent is
    /// owed only when there is something to put after it.
    #[test]
    fn a_trailing_newline_gets_no_indent_after_it() {
        assert_eq!(trailing_newline::Vars.render(), "start\n    <a>\nend");
    }

    #[test]
    fn one_line_partials_work_indented_or_not() {
        assert_eq!(indented_one_line::Vars.render(), "start\n    <a>end");
        assert_eq!(at_margin::Vars.render(), "start\n<a>end");
    }

    /// Anything else on the line and the partial is ordinary: no indent, and the newline stays.
    #[test]
    fn anything_else_on_the_line_cancels_it() {
        assert_eq!(inline::Vars.render(), "start\n  x<a>\n<b>\n<c>\nend");
    }

    /// Indents **accumulate**: a partial included from inside another standalone partial is
    /// indented by both. Checked against handlebars.js, which composes them the same way.
    #[test]
    fn nested_standalone_partials_add_their_indents() {
        assert_eq!(nested::Vars.render(), "start\n    X\n      <a>Yend");
    }

    #[test]
    fn the_end_of_the_template_ends_the_line() {
        assert_eq!(at_eof::Vars.render(), "start\n    <a>\n    <b>\n    <c>");
    }

    /// An empty partial leaves nothing at all — not even the indent it would have been given.
    #[test]
    fn an_empty_partial_leaves_nothing() {
        assert_eq!(of_nothing::Vars.render(), "start\nend");
    }
}
