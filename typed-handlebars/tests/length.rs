//! `{{ rows.length }}` — how many items a list holds.
//!
//! In handlebars.js `length` is an ordinary property lookup that happens to land on the one JS
//! arrays carry, so a designer writes it without thinking. Here it is read off the path instead:
//! `x.length` says `x` is a list and asks for its count, rather than declaring a record with a
//! field called `length`.
//!
//! The distinction the counts turn on is absent versus empty. A list that was never set counts as
//! nothing, exactly as an undefined value does in handlebars.js; a list with no items counts `0`.

#[test]
fn a_list_reports_how_many_items_it_holds() {
    mod template {
        typed_handlebars::str!("test", r#"<p>{{ rows.length }}</p>"#);
    }
    assert_eq!(template::test(vec!["a", "b", "c"]).render(), "<p>3</p>");
    assert_eq!(template::test(Vec::<&str>::new()).render(), "<p>0</p>");
}

/// The case that made this worth having: counted and iterated in the same template. This used to be
/// a compile error saying `rows` could not be both a list and a record — which handlebars.js says
/// it can.
#[test]
fn a_list_can_be_counted_and_iterated() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"<p>{{ rows.length }} rows</p><ul>{{#each rows}}<li>{{ name }}</li>{{/each}}</ul>"#
        );
    }
    let rows = vec![
        template::test::RowsItem::new("King"),
        template::test::RowsItem::new("Tubby"),
    ];
    assert_eq!(
        template::test(rows).render(),
        "<p>2 rows</p><ul><li>King</li><li>Tubby</li></ul>"
    );
}

/// A count is a number, so zero is falsy and anything else is truthy — Handlebars truthiness
/// applied to the count rather than to the list.
#[test]
fn a_count_can_be_tested() {
    mod template {
        typed_handlebars::str!("test", r#"{{#if rows.length}}some{{else}}none{{/if}}"#);
    }
    assert_eq!(template::test(vec![1]).render(), "some");
    assert_eq!(template::test(Vec::<u8>::new()).render(), "none");
}

/// Absent is not empty. handlebars.js writes nothing for `undefined.length` and `0` for `[].length`,
/// and a variable a builder never set is the undefined one.
#[test]
fn an_unset_list_counts_as_nothing_rather_than_zero() {
    mod counted {
        typed_handlebars::str!("test", r#"[{{ rows.length }}]"#);
    }
    mod iterated {
        typed_handlebars::str!(
            "test",
            r#"[{{ rows.length }}{{#each rows}}{{ name }}{{/each}}]"#
        );
    }
    assert_eq!(counted::test::Builder::new().render(), "[]");
    assert_eq!(
        counted::test::Builder::new().rows(vec![0; 2]).render(),
        "[2]"
    );
    assert_eq!(
        counted::test::Builder::new()
            .rows(Vec::<u8>::new())
            .render(),
        "[0]"
    );

    // The same holds when the list is iterated as well, where the unset value has to name its
    // item type.
    assert_eq!(iterated::test::Builder::new().render(), "[]");
    assert_eq!(
        iterated::test::Builder::new()
            .rows(vec![iterated::test::RowsItem::new("King")])
            .render(),
        "[1King]"
    );
}

/// An unset list is still falsy, and still iterates as an empty one — counting it changed neither.
#[test]
fn an_unset_list_is_still_empty_everywhere_else() {
    mod template {
        typed_handlebars::str!(
            "test",
            r#"[{{#if rows}}y{{/if}}{{#each rows}}{{this}}{{else}}none{{/each}}]"#
        );
    }
    assert_eq!(template::test::Builder::new().render(), "[none]");
}

#[test]
fn a_count_works_on_whatever_holds_the_list() {
    mod template {
        typed_handlebars::str!("test", r#"{{ rows.length }}"#);
    }
    let owned = vec![1, 2];
    assert_eq!(template::test(&owned).render(), "2");
    assert_eq!(template::test(["a", "b", "c"]).render(), "3");
    assert_eq!(template::test(&owned[..1]).render(), "1");
}

#[test]
fn a_list_inside_a_record_can_be_counted() {
    mod template {
        typed_handlebars::str!("test", r#"{{ page.rows.length }}"#);
    }
    assert_eq!(
        template::test(template::test::Page::new(vec!["a", "b"])).render(),
        "2"
    );
}

/// The count reaches out of a loop like any other value.
#[test]
fn a_count_resolves_through_scopes() {
    mod parent {
        typed_handlebars::str!("test", r#"{{#each rows}}[{{ ../rows.length }}]{{/each}}"#);
    }
    assert_eq!(parent::test(vec!["a", "b"]).render(), "[2][2]");
}

/// Counting the loop item itself, by alias or as `{{this}}` — which says the items are lists, and
/// is the one case where the count lands on a scope rather than on a field of one.
#[test]
fn an_each_item_can_be_counted() {
    mod aliased {
        typed_handlebars::str!(
            "test",
            r#"{{#each grids as |grid|}}[{{ grid.length }}]{{/each}}"#
        );
    }
    mod bare {
        typed_handlebars::str!("test", r#"{{#each grids}}[{{ this.length }}]{{/each}}"#);
    }
    let grids = vec![vec!["a", "b"], vec![]];
    assert_eq!(aliased::test(grids.clone()).render(), "[2][0]");
    assert_eq!(bare::test(grids).render(), "[2][0]");
}

/// `{{ 3 }}` is a literal in Handlebars, and `{{ length }}` on its own is an ordinary variable
/// called `length` — only a `.length` suffix means a count.
#[test]
fn a_bare_length_is_an_ordinary_variable() {
    mod template {
        typed_handlebars::str!("test", r#"{{ length }}"#);
    }
    assert_eq!(template::test("spool").render(), "spool");
}
