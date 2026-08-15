//! What an absent value renders as.
//!
//! handlebars.js writes nothing at all for null or undefined, so an `Option` that is `None` writes
//! nothing here too. That is the whole of it — nothing in the template says so, and nothing at the
//! call site has to unwrap, default or `as_deref` first.
//!
//! `false` and `0` are *not* absent in handlebars.js — they render as `false` and `0` — so they are
//! covered here too, to keep the line in the right place.

#[test]
fn none_renders_as_nothing() {
    mod template {
        typed_handlebars::str!("test", r#"<td>{{ guessed_datetime }}</td>"#);
    }
    let absent: Option<String> = None;
    assert_eq!(template::test(absent).render(), "<td></td>");
}

#[test]
fn some_renders_what_it_holds() {
    mod template {
        typed_handlebars::str!("test", r#"<td>{{ guessed_datetime }}</td>"#);
    }
    assert_eq!(
        template::test(Some("2026-08-15")).render(),
        "<td>2026-08-15</td>"
    );
}

/// The value inside a `Some` is escaped exactly as a bare value is: unwrapping happens on the way
/// to the escaper, not around it.
#[test]
fn some_is_escaped_and_raw_is_not() {
    mod escaped {
        typed_handlebars::str!("test", r#"{{ note }}"#);
    }
    mod raw {
        typed_handlebars::str!("test", r#"{{{ note }}}"#);
    }
    assert_eq!(
        escaped::test(Some("<b>hi</b>")).render(),
        "&lt;b&gt;hi&lt;/b&gt;"
    );
    assert_eq!(raw::test(Some("<b>hi</b>")).render(), "<b>hi</b>");
    assert_eq!(raw::test(None::<&str>).render(), "");
}

/// A row read from a database is usually still owned by the caller, so the nullable column arrives
/// as a reference.
#[test]
fn an_option_can_be_passed_by_reference() {
    mod template {
        typed_handlebars::str!("test", r#"<td>{{ note }}</td>"#);
    }
    struct Row {
        note: Option<String>,
    }
    let row = Row {
        note: Some("kept".into()),
    };
    let empty = Row { note: None };
    assert_eq!(template::test(&row.note).render(), "<td>kept</td>");
    assert_eq!(template::test(&empty.note).render(), "<td></td>");
}

#[test]
fn any_displayable_type_can_be_optional() {
    mod template {
        typed_handlebars::str!("test", r#"{{ a }}|{{ b }}|{{ c }}"#);
    }
    assert_eq!(
        template::test(Some(42), None::<f64>, Some(String::from("s"))).render(),
        "42||s"
    );
}

/// Testing a variable still does not stop you printing it — the promise the crate makes for every
/// other type, now kept for `Option` as well.
#[test]
fn an_option_can_be_both_tested_and_printed() {
    mod template {
        typed_handlebars::str!("test", r#"{{#if note}}<i>{{ note }}</i>{{else}}-{{/if}}"#);
    }
    assert_eq!(template::test(Some("here")).render(), "<i>here</i>");
    assert_eq!(template::test(None::<&str>).render(), "-");
}

#[test]
fn a_record_field_can_be_absent() {
    mod template {
        typed_handlebars::str!("test", r#"{{ person.name }}/{{ person.nickname }}"#);
    }
    assert_eq!(
        template::test(template::test::Person::new("King", None::<&str>)).render(),
        "King/"
    );
    assert_eq!(
        template::test(template::test::Person::new("King", Some("Tubby"))).render(),
        "King/Tubby"
    );
}

/// The case the whole change is for: a list of rows with a nullable column.
#[test]
fn a_nullable_column_renders_across_a_loop() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each rows}}<tr><td>{{ id }}</td><td>{{ guessed_datetime }}</td></tr>{{/each}}"#
        );
    }
    let rows = vec![
        template::test::RowsItem::new(1, Some("2026-08-15")),
        template::test::RowsItem::new(2, None),
    ];
    assert_eq!(
        template::test(rows).render(),
        "<tr><td>1</td><td>2026-08-15</td></tr><tr><td>2</td><td></td></tr>"
    );
}

/// A list of bare `Option`s, printed with `{{this}}`. The loop hands its body a reference, so this
/// is the case that decides whether a written value can reach its `Render` impl through one.
#[test]
fn a_list_of_options_renders_item_by_item() {
    mod template {
        typed_handlebars::str!("test", r#"{{#each tags}}[{{this}}]{{/each}}"#);
    }
    let tags: Vec<Option<String>> = vec![Some("a".into()), None, Some("c".into())];
    assert_eq!(template::test(tags).render(), "[a][][c]");
}

/// Every scope a value can be written from resolves to a different Rust expression, so an `Option`
/// is checked through the ones that are not a plain field of `self`.
#[test]
fn an_option_renders_from_any_scope() {
    mod with {
        typed_handlebars::str!("test", r#"{{#with person}}[{{ nickname }}]{{/with}}"#);
    }
    mod parent {
        typed_handlebars::str!(
            "test",
            r#"{{#each rows}}[{{ ../caption }}{{ id }}]{{/each}}"#
        );
    }
    mod named {
        typed_handlebars::str!(
            "test",
            r#"{{#each rows as |row|}}[{{ row.note }}]{{/each}}"#
        );
    }
    assert_eq!(
        with::test(with::test::Person::new(None::<&str>)).render(),
        "[]"
    );
    assert_eq!(
        parent::test(vec![parent::test::RowsItem::new(1)], None::<&str>).render(),
        "[1]"
    );
    assert_eq!(
        named::test(vec![
            named::test::RowsItem::new(Some("n")),
            named::test::RowsItem::new(None)
        ])
        .render(),
        "[n][]"
    );
}

/// `{{#if}}` on an `Option` was already handlebars.js-correct, and printing it does not change
/// that: `None` is falsy, `Some` is truthy whatever it wraps — including `Some("")` and `Some(0)`.
#[test]
fn presence_rather_than_contents_decides_truthiness() {
    mod template {
        typed_handlebars::str!("test", r#"{{#if note}}yes{{else}}no{{/if}}"#);
    }
    assert_eq!(template::test(Some("")).render(), "yes");
    assert_eq!(template::test(Some(0)).render(), "yes");
    assert_eq!(template::test(None::<&str>).render(), "no");
}

/// Absent means null and undefined, and nothing else. `false` and `0` are values, and handlebars.js
/// writes them out.
#[test]
fn false_and_zero_are_written_not_skipped() {
    mod template {
        typed_handlebars::str!("test", r#"{{ flag }}/{{ count }}"#);
    }
    assert_eq!(template::test(false, 0).render(), "false/0");
}

#[test]
fn a_builder_takes_an_option_like_any_other_value() {
    mod template {
        typed_handlebars::str!("test", r#"<td>{{ id }}{{ note }}</td>"#);
    }
    let absent: Option<String> = None;
    assert_eq!(
        template::test::Builder::new().id(7).note(absent).render(),
        "<td>7</td>"
    );
    assert_eq!(
        template::test::Builder::new()
            .id(7)
            .note(Some("n"))
            .render(),
        "<td>7n</td>"
    );
    // A variable left out entirely is absent in the same way.
    assert_eq!(template::test::Builder::new().id(7).render(), "<td>7</td>");
}
