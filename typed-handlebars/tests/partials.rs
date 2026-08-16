//! Partials, which need real files on disk and so cannot live in the `str!` unit tests.
//!
//! A partial is spliced into whoever includes it, so it renders against the surrounding context —
//! the same rule handlebars.js follows.

mod templates {
    typed_handlebars::directory!("tests/templates/");
}

#[test]
fn a_partial_renders_against_the_context_it_was_included_from() {
    // `row.hbs` writes {{ name }}; included inside {{#each rows}}, that means the current row.
    assert_eq!(
        templates::page::Vars {
            title: "Dub",
            rows: vec![
                templates::page::RowsItem {
                    id: 1,
                    name: "King"
                },
                templates::page::RowsItem {
                    id: 2,
                    name: "Tubby"
                },
            ],
        }
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
        templates::page::builder()
            .title("Dub")
            .rows(vec![
                templates::page::RowsItem::builder().name("King").build()
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
        templates::row::Vars {
            id: 9,
            name: "Standalone"
        }
        .render(),
        "<li id=\"r9\">Standalone</li>"
    );
    assert_eq!(
        templates::header::Vars {
            title: "Just a heading"
        }
        .render(),
        "<h1>Just a heading</h1>"
    );
}

/// A partial that includes a partial.
#[test]
fn partials_nest() {
    assert_eq!(
        templates::wrapper::Vars {
            title: "Dub",
            rows: vec![templates::wrapper::RowsItem {
                id: 1,
                name: "King"
            }],
        }
        .render(),
        "<div><h1>Dub</h1><ul><li id=\"r1\">King</li></ul></div>"
    );
}

/// The directory layout is something the template author expressed, so it becomes the module
/// layout. Two templates called `row` in different directories used to be
/// `E0428: the name row is defined multiple times`, with nothing naming either file.
#[test]
fn subdirectories_become_modules() {
    assert_eq!(
        templates::admin::row::Vars { name: "King" }.render(),
        "<tr class=\"admin\"><td>King</td></tr>"
    );
    assert_eq!(
        templates::public::row::Vars { name: "King" }.render(),
        "<tr><td>King</td></tr>"
    );
}

/// Template file names come from whoever writes the templates, so a file called `mod.hbs` or
/// `2col.hbs` has to work. Both used to be a proc-macro panic or `expected identifier, found
/// keyword`.
#[test]
fn template_file_names_may_be_rust_keywords() {
    assert_eq!(
        templates::awkward::mod_::Vars { x: "x" }.render(),
        "<p>x</p>"
    );
    assert_eq!(
        templates::awkward::_2col::Vars { x: "x" }.render(),
        "<p>x</p>"
    );
}

/// Values from a partial are escaped like any other, since the partial is generated code rather
/// than a rendered string being written back in.
#[test]
fn values_written_by_a_partial_are_escaped() {
    assert_eq!(
        templates::row::Vars {
            id: 1,
            name: "Tom & Jerry"
        }
        .render(),
        "<li id=\"r1\">Tom &amp; Jerry</li>"
    );
}
