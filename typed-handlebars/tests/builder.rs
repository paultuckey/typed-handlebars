//! The generated builder: named setters, any order, and unset meaning empty.
//!
//! An integration test, so the macro is reached the way a consumer reaches it — through
//! `typed_handlebars::str!` rather than `crate::str!`. That also covers the path resolution these
//! tests used to sit on the wrong side of: generated code names the runtime crate absolutely, and
//! from here that name has to resolve to a dependency rather than to the crate under test.

/// Handlebars renders an undefined variable as nothing, and the builder does the same: you set
/// what you have.
#[test]
fn the_builder_leaves_unset_variables_empty() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"<h1>{{title}}</h1><p>{{body}}</p>"#
        );
    }
    assert_eq!(
        template::test::Builder::new().render(),
        "<h1></h1><p></p>",
        "nothing set at all"
    );
    assert_eq!(
        template::test::Builder::new().title("Dub").render(),
        "<h1>Dub</h1><p></p>",
        "only one of the two set"
    );
}

/// An unset list has no items, and an unset condition is false — again matching Handlebars.
#[test]
fn unset_lists_and_conditions_are_empty() {
    mod list {
        typed_handlebars::str!("test", r#"[{{#each rows}}<li>{{name}}</li>{{/each}}]"#);
    }
    assert_eq!(list::test::Builder::new().render(), "[]");

    mod conditional {
        typed_handlebars::str!("test", r#"[{{#if shown}}yes{{/if}}]"#);
    }
    assert_eq!(conditional::test::Builder::new().render(), "[]");
    assert_eq!(
        conditional::test::Builder::new().shown(true).render(),
        "[yes]"
    );
}

/// A record left out renders as a record whose own fields are all empty.
#[test]
fn an_unset_record_is_empty_all_the_way_down() {
    mod template {
        typed_handlebars::str!("test", r#"[{{person.first}}|{{person.last}}]"#);
    }
    assert_eq!(template::test::Builder::new().render(), "[|]");
    assert_eq!(
        template::test::Builder::new()
            .person(template::test::PersonBuilder::new().first("King").build())
            .render(),
        "[King|]",
        "a record can itself be partly set"
    );
}

/// The wiring API: named setters, so nothing depends on argument order and every name comes
/// from autocomplete rather than being retyped from the template.
#[test]
fn the_builder_is_order_independent() {
    mod template {
        typed_handlebars::str!("test", r#"<p>{{firstname}} {{lastname}}</p>"#);
    }
    assert_eq!(
        template::test::Builder::new()
            .firstname("King")
            .lastname("Tubby")
            .render(),
        "<p>King Tubby</p>"
    );
    // …and the other way round.
    assert_eq!(
        template::test::Builder::new()
            .lastname("Tubby")
            .firstname("King")
            .render(),
        "<p>King Tubby</p>"
    );
}

/// Generated item types get builders too, so a nested shape is wired up the same way.
#[test]
fn the_builder_wires_up_lists() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"<h1>{{title}}</h1>{{#each rows}}<li>{{name}} {{email}}</li>{{/each}}"#
        );
    }
    let rows = vec![
        template::test::RowsItemBuilder::new()
            .name("King")
            .email("king@example.com")
            .build(),
        template::test::RowsItemBuilder::new()
            .email("tubby@example.com")
            .name("Tubby")
            .build(),
    ];
    let expected = "<h1>Dub</h1><li>King king@example.com</li><li>Tubby tubby@example.com</li>";

    assert_eq!(
        template::test::Builder::new()
            .title("Dub")
            .rows(&rows)
            .render(),
        expected
    );

    // `build` hands back the template value, which can be rendered more than once.
    let page = template::test::Builder::new()
        .rows(rows)
        .title("Dub")
        .build();
    assert_eq!(page.render(), expected);
    assert_eq!(page.render(), expected);
}
