//! Partials, which need real files on disk and so cannot live in the `str!` unit tests.
//!
//! A partial is spliced into whoever includes it, so it renders against the surrounding context —
//! the same rule handlebars.js follows.

mod templates {
    dry_handlebars::directory!("tests/templates/");
}

#[test]
fn a_partial_renders_against_the_context_it_was_included_from() {
    // `row.hbs` writes {{ name }}; included inside {{#each rows}}, that means the current row.
    assert_eq!(
        templates::page(
            "Dub",
            vec![
                templates::page_rows_item::new(1, "King"),
                templates::page_rows_item::new(2, "Tubby"),
            ],
        )
        .render(),
        "<h1>Dub</h1><ul><li id=\"r1\">King</li><li id=\"r2\">Tubby</li></ul>"
    );
}

/// A partial's variables become part of the including template's shape, so `page` asks for
/// `title` (from `header.hbs`) and rows of `id`/`name` (from `row.hbs`) without naming either file
/// in Rust.
#[test]
fn a_partial_contributes_to_the_builder_of_its_includer() {
    assert_eq!(
        templates::page_builder::new()
            .title("Dub")
            .rows(vec![
                templates::page_rows_item_builder::new()
                    .name("King")
                    .build()
            ])
            .render(),
        "<h1>Dub</h1><ul><li id=\"r\">King</li></ul>",
        "an unset id renders empty, as any unset variable does"
    );
}

/// Being a partial does not stop a template being used on its own.
#[test]
fn a_partial_still_has_its_own_type() {
    assert_eq!(
        templates::row(9, "Standalone").render(),
        "<li id=\"r9\">Standalone</li>"
    );
    assert_eq!(
        templates::header("Just a heading").render(),
        "<h1>Just a heading</h1>"
    );
}

/// A partial that includes a partial.
#[test]
fn partials_nest() {
    assert_eq!(
        templates::wrapper("Dub", vec![templates::wrapper_rows_item::new(1, "King")]).render(),
        "<div><h1>Dub</h1><ul><li id=\"r1\">King</li></ul></div>"
    );
}

/// Values from a partial are escaped like any other, since the partial is generated code rather
/// than a rendered string being written back in.
#[test]
fn values_written_by_a_partial_are_escaped() {
    assert_eq!(
        templates::row(1, "Tom & Jerry").render(),
        "<li id=\"r1\">Tom &amp; Jerry</li>"
    );
}
