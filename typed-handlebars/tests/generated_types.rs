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
        template::test::Vars {
            rows: vec![
                template::test::RowsItem {
                    name: "King",
                    email: "king@example.com"
                },
                template::test::RowsItem {
                    name: "Tubby",
                    email: "tubby@example.com"
                },
            ]
        }
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
    assert_eq!(
        template::test::Vars {
            tags: vec!["a", "b"]
        }
        .render(),
        "[a][b]"
    );
    assert_eq!(
        template::test::Vars { tags: [1, 2, 3] }.render(),
        "[1][2][3]"
    );
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
        template::test::RowsItem {
            cells: vec![
                template::test::RowsItemCellsItem { value: 1 },
                template::test::RowsItemCellsItem { value: 2 },
            ],
        },
        template::test::RowsItem {
            cells: vec![template::test::RowsItemCellsItem { value: 3 }],
        },
    ];
    assert_eq!(
        template::test::Vars { rows }.render(),
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
        template::test::Vars {
            person: template::test::Person {
                firstname: "King",
                lastname: "Tubby"
            }
        }
        .render(),
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
        template::test::Vars {
            type_: "a",
            match_: "b",
            fn_: "c",
            self_: "d",
            crate_: "e",
            loop_: "f"
        }
        .render(),
        "[a][b][c][d][e][f]"
    );
    // The builder renames them the same way, so autocomplete still finds them.
    assert_eq!(
        template::test::builder().type_("a").self_("d").render(),
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
        template::test::Vars {
            _2nd: "silver",
            _42: "answer"
        }
        .render(),
        "[silver][answer]"
    );
}

/// Testing an `{{#each}}` item *itself* bounds it by `Truthy`, as testing a named field does.
/// It used to bound nothing, so `{{#if this}}` inside a loop failed with
/// `error[E0277]: the trait bound 'T0: Truthy' is not satisfied` — a Rust error against a template
/// that is perfectly good Handlebars.
#[test]
fn testing_the_item_itself_bounds_it() {
    mod printed_too {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "{{#each xs}}{{#if this}}[{{this}}]{{/if}}{{/each}}"
        );
    }
    assert_eq!(
        printed_too::test::Vars { xs: vec![1, 0, 2] }.render(),
        "[1][2]"
    );

    mod negated {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "{{#each xs}}{{#unless this}}n{{/unless}}{{/each}}"
        );
    }
    assert_eq!(negated::test::Vars { xs: vec![1, 0] }.render(), "n");

    // An alias reaches the same scope, so it needs the same bound — a fix keyed on the literal
    // `this` would miss this half.
    mod through_an_alias {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "{{#each xs as |x|}}{{#if x}}[{{x}}]{{/if}}{{/each}}"
        );
    }
    assert_eq!(
        through_an_alias::test::Vars { xs: vec!["a", ""] }.render(),
        "[a]"
    );

    // Nested loops each bound their own item.
    mod nested {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "{{#each rows}}{{#each cells}}{{#if this}}[{{this}}]{{/if}}{{/each}};{{/each}}"
        );
    }
    assert_eq!(
        nested::test::Vars {
            rows: vec![
                nested::test::RowsItem { cells: vec![1, 0] },
                nested::test::RowsItem { cells: vec![2] },
            ]
        }
        .render(),
        "[1];[2];"
    );
}

/// An item that is only tested needs no `Display`, in the same way a named field that is only
/// tested does not. The bound follows what the template asks for rather than being applied to
/// every item.
///
/// `OnlyTruthy` is the proof: it has no `Display` of its own, so this stops compiling the moment
/// the item is asked to be printable. Implementing `Truthy` by hand is not something a consumer
/// should ever need to do — it is done here precisely because it makes the bound observable.
#[test]
fn an_item_that_is_only_tested_needs_no_display() {
    struct OnlyTruthy(bool);

    impl typed_handlebars::Truthy for OnlyTruthy {
        fn is_truthy(&self) -> bool {
            self.0
        }
    }

    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            "{{#each xs}}{{#if this}}y{{/if}}{{/each}}"
        );
    }
    assert_eq!(
        template::test::Vars {
            xs: vec![OnlyTruthy(true), OnlyTruthy(false)]
        }
        .render(),
        "y"
    );
}

/// A variable is named by whoever writes the `.hbs`, so `{{ builder.name }}` is ordinary
/// Handlebars — and it camel-cases straight onto a type the module already generates. It used to be
/// a wall of `E0428: the name Builder is defined multiple times` against a template that is not
/// wrong about anything, which is exactly the Rust error a template author cannot read.
///
/// Names are handed out in template order, and whatever finds its name taken takes a trailing
/// underscore instead — the same escape a Rust keyword gets. The module's own API is reserved
/// ahead of everything, so `Vars` and `Builder` always mean what a caller expects.
#[test]
fn a_variable_may_be_called_vars_or_builder() {
    mod clashes_with_the_type {
        typed_handlebars::str!("test", r#"[{{ vars.x }}]"#);
    }
    assert_eq!(
        clashes_with_the_type::test::Vars {
            vars: clashes_with_the_type::test::Vars_ { x: 1 }
        }
        .render(),
        "[1]"
    );

    mod clashes_with_the_builder {
        typed_handlebars::str!("test", r#"[{{ builder.x }}]"#);
    }
    assert_eq!(
        clashes_with_the_builder::test::Vars {
            builder: clashes_with_the_builder::test::Builder_ { x: 2 }
        }
        .render(),
        "[2]"
    );

    // An escaped type is a type like any other, builder included.
    assert_eq!(
        clashes_with_the_type::test::builder()
            .vars(clashes_with_the_type::test::Vars_::builder().x(3).build())
            .render(),
        "[3]"
    );
}

/// Two variables can also collide with each other, once one of them camel-cases onto a name the
/// other one's type brings with it. First mentioned keeps the name.
#[test]
fn generated_names_give_way_in_template_order() {
    // `rows_item` is written before `{{#each rows}}`, so it is the loop's item type that steps
    // aside rather than the record.
    mod item {
        typed_handlebars::str!(
            "test",
            r#"[{{ rows_item.x }}|{{#each rows}}{{y}}{{/each}}]"#
        );
    }
    assert_eq!(
        item::test::Vars {
            rows_item: item::test::RowsItem { x: 1 },
            rows: vec![item::test::RowsItem_ { y: 2 }],
        }
        .render(),
        "[1|2]"
    );

    // `person_builder` takes `PersonBuilder`, so `person`'s own builder has nowhere to go — and it
    // is the record that moves, since a type and its builder have to stay together.
    mod builder {
        typed_handlebars::str!("test", r#"[{{ person_builder.x }}|{{ person.y }}]"#);
    }
    assert_eq!(
        builder::test::Vars {
            person_builder: builder::test::PersonBuilder { x: 3 },
            person: builder::test::Person_ { y: 4 },
        }
        .render(),
        "[3|4]"
    );
}
