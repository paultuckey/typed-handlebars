//! `{{#each}}`: what it iterates, what it binds, and the `@…` variables it supplies.
//!
//! An integration test, so the macro is reached the way a consumer reaches it — through
//! `typed_handlebars::str!` rather than `crate::str!`. That also covers the path resolution these
//! tests used to sit on the wrong side of: generated code names the runtime crate absolutely, and
//! from here that name has to resolve to a dependency rather than to the crate under test.

/// `{{@index}}` inside `{{#each}}`. Mirrored in `reference-ts`.
#[test]
fn each_index() {
    mod template {
        typed_handlebars::str!("test", r#"{{#each rows}}{{@index}}:{{name}} {{/each}}"#);
    }
    assert_eq!(
        template::test(vec![
            template::test::RowsItem::new("a"),
            template::test::RowsItem::new("b"),
        ])
        .render(),
        "0:a 1:b "
    );
}

/// `{{@first}}` and `{{@last}}` are answered from the same counter `{{@index}}` uses, so they
/// render as `true`/`false` and a one-item list is both. Mirrored in `reference-ts`.
#[test]
fn each_first_and_last() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each xs}}[{{@first}},{{@last}},{{@index}}]{{/each}}"#
        );
    }
    assert_eq!(
        template::test(vec![1, 2, 3]).render(),
        "[true,false,0][false,false,1][false,true,2]"
    );

    mod one {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each xs}}[{{@first}},{{@last}}]{{/each}}"#
        );
    }
    assert_eq!(one::test(vec![9]).render(), "[true,true]");
}

/// Both work as conditions as well as values — `{{#unless @last}}` between items is the reason
/// most templates want them.
#[test]
fn each_first_and_last_are_conditions_too() {
    mod separator {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each xs}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}"#
        );
    }
    assert_eq!(separator::test(vec![1, 2, 3]).render(), "1, 2, 3");

    mod first_only {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each xs}}{{#if @first}}F{{else}}-{{/if}}{{/each}}"#
        );
    }
    assert_eq!(first_only::test(vec![1, 2, 3]).render(), "F--");
}

/// An `@…` is supplied by the loop, and blocks that supply nothing are transparent to it — so
/// reading one from inside a nested `{{#if}}` or `{{#with}}` works. It used to fail with
/// `@index not expected`, which made whether an `@…` resolved depend on nothing more than
/// when the enclosing block happened to be pushed. Mirrored in `reference-ts`.
#[test]
fn a_private_is_visible_through_blocks_that_supply_nothing() {
    mod inside_if {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each xs}}{{#if on}}[{{@index}}]{{/if}}{{/each}}"#
        );
    }
    assert_eq!(
        inside_if::test(vec![
            inside_if::test::XsItem::new(true),
            inside_if::test::XsItem::new(false),
        ])
        .render(),
        "[0]"
    );

    mod inside_with {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each rows}}{{#with p}}[{{@index}}:{{n}}]{{/with}}{{/each}}"#
        );
    }
    assert_eq!(
        inside_with::test(vec![
            inside_with::test::RowsItem::new(inside_with::test::RowsItemP::new("a")),
            inside_with::test::RowsItem::new(inside_with::test::RowsItemP::new("b")),
        ])
        .render(),
        "[0:a][1:b]"
    );

    // The chained form, which is where this was most visible: `{{#if @last}}` resolved and
    // `{{else if @first}}` did not, purely because of when the `{{#if}}` was pushed.
    mod chained {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each xs}}{{#if @last}}L{{else if @first}}F{{else}}m{{/if}}{{/each}}"#
        );
    }
    assert_eq!(chained::test(vec![1, 2, 3]).render(), "FmL");
}

/// `../` on a private steps out one **loop**, not one scope, so an intervening `{{#if}}` or
/// `{{#with}}` does not absorb it. Checked against handlebars.js, which agrees.
#[test]
fn a_parent_private_steps_out_one_loop_not_one_block() {
    mod through_if {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each rows}}{{#each cells}}{{#if on}}{{@../index}}{{/if}}{{/each}};{{/each}}"#
        );
    }
    let rows = vec![
        through_if::test::RowsItem::new(vec![
            through_if::test::RowsItemCellsItem::new(true),
            through_if::test::RowsItemCellsItem::new(true),
        ]),
        through_if::test::RowsItem::new(vec![through_if::test::RowsItemCellsItem::new(true)]),
    ];
    assert_eq!(through_if::test(rows).render(), "00;1;");
}

/// `../` steps out to the enclosing loop, as it does for `@index`.
#[test]
fn each_first_reaches_the_enclosing_loop() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each rows}}{{#each cells}}{{@../first}}/{{@first}};{{/each}}|{{/each}}"#
        );
    }
    let rows = vec![
        template::test::RowsItem::new(vec![1, 2]),
        template::test::RowsItem::new(vec![3]),
    ];
    assert_eq!(
        template::test(rows).render(),
        "true/true;true/false;|false/true;|"
    );
}

/// A block alias is a local in the generated code, and so is the loop counter. They used to be
/// able to collide: `as |i|` produced `i_0`, shadowed the `i_0` counter, and the increment then
/// landed on the loop item — which the template author saw as `E0368` against their `.hbs`.
#[test]
fn a_block_alias_cannot_shadow_the_loop_counter() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each xs as |i|}}{{i}}:{{@index}};{{/each}}"#
        );
    }
    assert_eq!(template::test(vec![7, 8]).render(), "7:0;8:1;");

    // The mirror of that: an alias named after a private is a plain variable, not a reference
    // to `@first`.
    mod aliased_first {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each xs as |first|}}{{first}};{{/each}}"#
        );
    }
    assert_eq!(aliased_first::test(vec![4, 5]).render(), "4;5;");
}

/// A comment inside an `{{#each}}` used to hang the compiler outright: the scan that decides
/// whether the block needs a counter skipped comments with a `continue`, in a loop that
/// advances at the bottom.
#[test]
fn a_comment_inside_each_terminates() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each xs}}{{! why }}[{{this}}]{{/each}}"#
        );
    }
    assert_eq!(template::test(vec![1, 2]).render(), "[1][2]");

    // `{{else}}` is found by the same kind of scan, which had the same bug.
    mod with_else {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each xs}}{{! why }}[{{this}}]{{else}}none{{/each}}"#
        );
    }
    assert_eq!(with_else::test(Vec::<i32>::new()).render(), "none");
}

/// `{{else}}` inside `{{#each}}` covers the empty list. Mirrored in `reference-ts`.
#[test]
fn each_else() {
    mod template {
        typed_handlebars::str!("test", r#"{{#each rows}}{{name}}{{else}}none{{/each}}"#);
    }
    assert_eq!(
        template::test(vec![template::test::RowsItem::new("a")]).render(),
        "a"
    );
    assert_eq!(template::test::Builder::new().render(), "none");
}

#[test]
fn for_helper() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"<div>{{#each authors}}<p>Hello {{first_name}}</p>{{/each}}</div>"#
        );
    }
    assert_eq!(
        template::test(vec![template::test::AuthorsItem::new("King")]).render(),
        //language=html
        "<div><p>Hello King</p></div>"
    );
}

#[test]
fn each_can_reach_the_enclosing_scope() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each rows}}<li>{{name}} of {{../company}}</li>{{/each}}"#
        );
    }
    assert_eq!(
        template::test(vec![template::test::RowsItem::new("King")], "Studio One").render(),
        //language=html
        "<li>King of Studio One</li>"
    );
}

#[test]
fn each_accepts_a_named_item() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each rows as |row|}}<li>{{row.name}}</li>{{/each}}"#
        );
    }
    assert_eq!(
        template::test(vec![template::test::RowsItem::new("King")]).render(),
        //language=html
        "<li>King</li>"
    );
}

/// Rendering borrows, so a caller can pass a list they still own — no clone, no giving up the
/// data.
#[test]
fn each_accepts_borrowed_lists() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each rows}}<li>{{name}}</li>{{/each}}"#
        );
    }
    let rows = vec![
        template::test::RowsItem::new("King"),
        template::test::RowsItem::new("Tubby"),
    ];
    let expected = "<li>King</li><li>Tubby</li>";

    assert_eq!(template::test(&rows).render(), expected);
    assert_eq!(template::test(rows.as_slice()).render(), expected);

    // The caller still owns it, and can hand it over afterwards if they want to.
    assert_eq!(rows.len(), 2);
    assert_eq!(template::test(rows).render(), expected);

    let array = [template::test::RowsItem::new("King")];
    assert_eq!(template::test(&array).render(), "<li>King</li>");
    assert_eq!(template::test(array).render(), "<li>King</li>");
}

/// Rendering borrows rather than consumes, so a template may walk the same list twice.
#[test]
fn the_same_list_can_be_iterated_twice() {
    mod template {
        typed_handlebars::str!(
            "test",
            //language=handlebars
            r#"{{#each rows}}{{name}}{{/each}}|{{#each rows}}{{name}}{{/each}}"#
        );
    }
    let page = template::test(vec![
        template::test::RowsItem::new("a"),
        template::test::RowsItem::new("b"),
    ]);
    assert_eq!(page.render(), "ab|ab");
    // …and the value is still usable afterwards.
    assert_eq!(page.render(), "ab|ab");
}
