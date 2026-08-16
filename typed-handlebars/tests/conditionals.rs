//! `{{#if}}`, `{{#unless}}`, `{{#with}}` and the `{{else}}` family, including the
//! Handlebars truthiness that decides which branch runs.
//!
//! An integration test, so the macro is reached the way a consumer reaches it — through
//! `typed_handlebars::str!` rather than `crate::str!`. That also covers the path resolution these
//! tests used to sit on the wrong side of: generated code names the runtime crate absolutely, and
//! from here that name has to resolve to a dependency rather than to the crate under test.

#[test]
fn if_helper() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"<div>{{#if has_author}}<h1>{{first_name}} {{last_name}}</h1>{{/if}}</div>"#
        );
    }
    assert_eq!(
        template::test::Vars {
            has_author: true,
            first_name: "King",
            last_name: "Tubby"
        }
        .render(),
        //language=html
        "<div><h1>King Tubby</h1></div>"
    );
    assert_eq!(
        template::test::Vars {
            has_author: false,
            first_name: "King",
            last_name: "Tubby"
        }
        .render(),
        //language=html
        "<div></div>"
    );
}

/// The idiom that could not be expressed at all before: test a variable and then print it.
/// Testing one no longer forces it to be a `bool`.
#[test]
fn a_variable_can_be_tested_and_printed() {
    mod template {
        typed_handlebars::str!("test", r#"[{{#if title}}{{title}}{{/if}}]"#);
    }
    assert_eq!(
        template::test::Vars {
            title: String::from("Dub")
        }
        .render(),
        "[Dub]"
    );
    assert_eq!(
        template::test::Vars {
            title: String::new()
        }
        .render(),
        "[]"
    );
    assert_eq!(template::test::Vars { title: "Dub" }.render(), "[Dub]");
    assert_eq!(template::test::Vars { title: "" }.render(), "[]");
    assert_eq!(template::test::Vars { title: 7 }.render(), "[7]");
    assert_eq!(template::test::Vars { title: 0 }.render(), "[]");
    assert_eq!(template::test::Vars { title: true }.render(), "[true]");
    assert_eq!(template::test::Vars { title: false }.render(), "[]");
}

/// What counts as falsy, following handlebars.js. Mirrored in `reference-ts`.
#[test]
fn falsiness_follows_handlebars() {
    mod template {
        typed_handlebars::str!("test", r#"[{{#if value}}yes{{/if}}]"#);
    }
    // Absent and false.
    assert_eq!(template::test::builder().render(), "[]");
    assert_eq!(template::test::Vars { value: false }.render(), "[]");
    assert_eq!(template::test::Vars { value: true }.render(), "[yes]");
    // Empty string and zero.
    assert_eq!(template::test::Vars { value: "" }.render(), "[]");
    assert_eq!(template::test::Vars { value: "x" }.render(), "[yes]");
    assert_eq!(template::test::Vars { value: 0 }.render(), "[]");
    assert_eq!(template::test::Vars { value: -1 }.render(), "[yes]");
    // Option is present or not, whatever it wraps.
    assert_eq!(
        template::test::Vars {
            value: None::<&str>
        }
        .render(),
        "[]"
    );
    assert_eq!(template::test::Vars { value: Some("") }.render(), "[yes]");
    // A list is falsy when it has no items.
    assert_eq!(
        template::test::Vars {
            value: Vec::<u8>::new()
        }
        .render(),
        "[]"
    );
    assert_eq!(template::test::Vars { value: vec![1] }.render(), "[yes]");
    // …including through a reference.
    let rows = vec![1];
    assert_eq!(template::test::Vars { value: &rows }.render(), "[yes]");
}

/// A list can be tested and then walked — the same variable serving both.
#[test]
fn a_list_can_be_tested_and_iterated() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#if rows}}<ul>{{#each rows}}<li>{{name}}</li>{{/each}}</ul>{{/if}}"#
        );
    }
    assert_eq!(
        template::test::Vars {
            rows: vec![template::test::RowsItem { name: "King" }]
        }
        .render(),
        //language=html
        "<ul><li>King</li></ul>"
    );
    assert_eq!(template::test::builder().render(), "", "no rows, no list");
}

#[test]
fn unless_helper() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"<div>{{#unless has_author}}<h1>Unknown</h1>{{/unless}}</div>"#
        );
    }
    assert_eq!(
        template::test::Vars { has_author: false }.render(),
        //language=html
        "<div><h1>Unknown</h1></div>"
    );
    assert_eq!(
        template::test::Vars { has_author: true }.render(),
        //language=html
        "<div></div>"
    );
}

/// `{{else}}` inside `{{#unless}}`. Mirrored in `reference-ts`.
#[test]
fn unless_else() {
    mod template {
        typed_handlebars::str!("test", r#"{{#unless a}}no{{else}}yes{{/unless}}"#);
    }
    assert_eq!(template::test::Vars { a: false }.render(), "no");
    assert_eq!(template::test::Vars { a: true }.render(), "yes");
}

#[test]
fn if_else_helper() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"<div>{{#if has_author}}<h1>{{first_name}}</h1>{{else}}<h1>Unknown</h1>{{/if}}</div>"#
        );
    }
    assert_eq!(
        template::test::Vars {
            has_author: true,
            first_name: "King"
        }
        .render(),
        //language=html
        r#"<div><h1>King</h1></div>"#
    );
    assert_eq!(
        template::test::Vars {
            has_author: false,
            first_name: "King"
        }
        .render(),
        //language=html
        r#"<div><h1>Unknown</h1></div>"#
    );
}

/// `{{else if}}` compiles to a Rust `else if`, which is what Handlebars means by it — the whole
/// chain shares the one `{{/if}}`. Mirrored in `reference-ts`.
#[test]
fn else_if_chains() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#if a}}A{{else if b}}B{{else}}C{{/if}}"#
        );
    }
    assert_eq!(template::test::Vars { a: true, b: false }.render(), "A");
    assert_eq!(template::test::Vars { a: false, b: true }.render(), "B");
    assert_eq!(template::test::Vars { a: false, b: false }.render(), "C");
}

/// A chain of any length still needs exactly one close, and the last `{{else}}` is optional.
#[test]
fn else_if_chains_more_than_once() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#if a}}A{{else if b}}B{{else if c}}C{{else}}D{{/if}}"#
        );
    }
    assert_eq!(
        template::test::Vars {
            a: false,
            b: false,
            c: true
        }
        .render(),
        "C"
    );
    assert_eq!(
        template::test::Vars {
            a: false,
            b: false,
            c: false
        }
        .render(),
        "D"
    );

    mod no_final_else {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#if a}}A{{else if b}}B{{/if}}"#
        );
    }
    assert_eq!(
        no_final_else::test::Vars { a: false, b: false }.render(),
        ""
    );
}

/// The tested variable gets a `Truthy` bound like any other condition, so `{{else if}}` is not
/// restricted to `bool` and the variable can still be printed inside its own branch.
#[test]
fn an_else_if_condition_is_truthy_not_bool() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#if a}}A{{else if name}}[{{name}}]{{else}}C{{/if}}"#
        );
    }
    assert_eq!(
        template::test::Vars {
            a: false,
            name: "King"
        }
        .render(),
        "[King]"
    );
    assert_eq!(
        template::test::Vars { a: false, name: "" }.render(),
        "C",
        "empty string is falsy"
    );
}

/// The chained helper decides the sense of the test, not the block it sits in: an `{{else if}}`
/// inside an `{{#unless}}` tests for truth, and `{{else unless}}` negates inside an `{{#if}}`.
/// Both checked against handlebars.js.
#[test]
fn a_chained_helper_sets_its_own_sense() {
    mod inside_unless {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#unless a}}U{{else if b}}B{{else}}C{{/unless}}"#
        );
    }
    assert_eq!(
        inside_unless::test::Vars { a: false, b: true }.render(),
        "U"
    );
    assert_eq!(inside_unless::test::Vars { a: true, b: true }.render(), "B");
    assert_eq!(
        inside_unless::test::Vars { a: true, b: false }.render(),
        "C"
    );

    mod else_unless {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#if a}}A{{else unless b}}B{{else}}C{{/if}}"#
        );
    }
    assert_eq!(else_unless::test::Vars { a: false, b: false }.render(), "B");
    assert_eq!(else_unless::test::Vars { a: false, b: true }.render(), "C");
}

/// The condition resolves in the scope the chain sits in, so a dotted path generates a record
/// and a chain inside an `{{#each}}` body reads the item.
#[test]
fn an_else_if_condition_resolves_in_its_own_scope() {
    mod dotted {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#if a}}A{{else if person.name}}B{{else}}C{{/if}}"#
        );
    }
    assert_eq!(
        dotted::test::Vars {
            a: false,
            person: dotted::test::Person { name: "King" }
        }
        .render(),
        "B"
    );

    mod in_each {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each rows}}{{#if hot}}H{{else if warm}}W{{else}}C{{/if}};{{/each}}"#
        );
    }
    assert_eq!(
        in_each::test::Vars {
            rows: vec![
                in_each::test::RowsItem {
                    hot: true,
                    warm: false
                },
                in_each::test::RowsItem {
                    hot: false,
                    warm: true
                },
                in_each::test::RowsItem {
                    hot: false,
                    warm: false
                },
            ]
        }
        .render(),
        "H;W;C;"
    );
}

/// `{{ else }}` is an `else`, not a variable called `else`. It used to be read as a variable by
/// the compiler and as an `else` by the type inference, so the two disagreed and the generated
/// code referred to a field that was never generated.
#[test]
fn else_may_be_spaced() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#if a}}A{{ else }}B{{/if}}"#
        );
    }
    assert_eq!(template::test::Vars { a: false }.render(), "B");

    // The word-boundary check that makes the above work must not swallow variables.
    mod elsewhere {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"[{{ elsewhere }}]"#
        );
    }
    assert_eq!(
        elsewhere::test::Vars { elsewhere: "town" }.render(),
        "[town]"
    );
}

/// Unset means falsy, so a chain reached through the builder behaves as an undefined variable
/// does in Handlebars rather than failing to compile.
#[test]
fn an_unset_else_if_condition_is_falsy() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#if a}}A{{else if b}}B{{else}}C{{/if}}"#
        );
    }
    assert_eq!(template::test::builder().render(), "C");
    assert_eq!(template::test::builder().b(true).render(), "B");
}

#[test]
fn with_helper() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"<div>{{#with author}}<h1>{{first_name}} {{last_name}}</h1>{{/with}}</div>"#
        );
    }
    assert_eq!(
        template::test::Vars {
            author: template::test::Author {
                first_name: "King",
                last_name: "Tubby"
            }
        }
        .render(),
        //language=html
        "<div><h1>King Tubby</h1></div>"
    );
}

/// Pins the one place this parts company with handlebars.js, so the divergence is visible
/// rather than folklore: handlebars.js skips a `{{#with}}` block whose subject is undefined,
/// but an unset record here is a record of empties, so the block still renders.
#[test]
fn with_renders_even_when_the_record_was_never_set() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"<div>{{#with author}}<h1>{{first_name}}</h1>{{/with}}</div>"#
        );
    }
    assert_eq!(
        template::test::builder().render(),
        //language=html
        "<div><h1></h1></div>",
        "handlebars.js would render <div></div> here"
    );
}
