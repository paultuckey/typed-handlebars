//! The types the macro infers from a template, and the names it gives them —
//! item types, nested records, and template names that are not legal Rust identifiers.
//!
//! An integration test, so the macro is reached the way a consumer reaches it — through
//! `typed_handlebars::str!` rather than `crate::str!`. That also covers the path resolution these
//! tests used to sit on the wrong side of: generated code names the runtime crate absolutely, and
//! from here that name has to resolve to a dependency rather than to the crate under test.

/// The template says `rows` is a list whose items have a `name`, so the macro generates the
/// item type. Nothing here declares a type or implements a trait.
#[test]
fn each_generates_the_item_type() {
    mod template {
        typed_handlebars::str!(
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
        typed_handlebars::str!("test", r#"{{#each tags}}[{{this}}]{{/each}}"#);
    }
    assert_eq!(template::test(vec!["a", "b"]).render(), "[a][b]");
    assert_eq!(template::test([1, 2, 3]).render(), "[1][2][3]");
}

#[test]
fn nested_each_generates_nested_item_types() {
    mod template {
        typed_handlebars::str!(
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
        typed_handlebars::str!("test", r#"{{person.firstname}} {{person.lastname}}"#);
    }
    assert_eq!(
        template::test(template::test::Person::new("King", "Tubby")).render(),
        "King Tubby"
    );
}

/// The person writing a template picks the variable names and has no reason to know what Rust
/// reserves. Every one of these used to be a proc-macro panic or a raw Rust syntax error.
#[test]
fn variable_names_may_be_rust_keywords() {
    mod template {
        typed_handlebars::str!(
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
        typed_handlebars::str!("test", r#"[{{ 2nd }}][{{ 42 }}]"#);
    }
    assert_eq!(
        template::test("silver", "answer").render(),
        "[silver][answer]"
    );
}
