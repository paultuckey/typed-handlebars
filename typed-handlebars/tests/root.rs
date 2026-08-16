//! `{{@root.…}}` — the template's own top-level context, from any depth.
//!
//! Not the same mechanism as `@index`, `@first` and `@last`. Those are loop state, supplied by the
//! `{{#each}}` a reference sits in and stepped outwards with `../`. `@root` is **absolute**: it
//! names the top of the template wherever it appears, which is why `{{@../root.title}}` reads the
//! same value as `{{@root.title}}` in handlebars.js and why nothing here walks outwards to find it.

#[test]
fn the_top_level_is_reachable_from_inside_a_loop() {
    mod template {
        typed_handlebars::str!("test", r#"{{#each rows}}[{{@root.title}}]{{/each}}"#);
    }
    assert_eq!(
        template::test::Vars {
            rows: vec![(), ()],
            title: "Dub"
        }
        .render(),
        "[Dub][Dub]"
    );
}

#[test]
fn and_from_inside_a_with() {
    mod template {
        typed_handlebars::str!("test", r#"{{#with person}}[{{@root.title}}]{{/with}}"#);
    }
    assert_eq!(
        template::test::Vars {
            person: template::test::Person {},
            title: "Dub"
        }
        .render(),
        "[Dub]"
    );
}

#[test]
fn and_from_the_top_level_itself() {
    mod template {
        typed_handlebars::str!("test", r#"[{{@root.title}}]"#);
    }
    assert_eq!(template::test::Vars { title: "Dub" }.render(), "[Dub]");
}

/// `../` has no effect on `@root`, so it is stripped rather than walked. Pinned because the
/// obvious implementation — reusing the outward walk the other `@…` variables need — would step
/// somewhere else here and be wrong in a way only a nested template would show.
#[test]
fn a_parent_prefix_makes_no_difference() {
    mod template {
        typed_handlebars::str!(
            "test",
            r#"{{#each rows}}{{#each inner}}[{{@root.title}}|{{@../root.title}}]{{/each}}{{/each}}"#
        );
    }
    let rows = vec![template::test::RowsItem {
        inner: vec![(), ()],
    }];
    assert_eq!(
        template::test::Vars { rows, title: "Dub" }.render(),
        "[Dub|Dub][Dub|Dub]"
    );
}

#[test]
fn a_path_beneath_the_root_can_be_any_depth() {
    mod template {
        typed_handlebars::str!(
            "test",
            r#"{{#each rows}}[{{@root.page.person.name}}]{{/each}}"#
        );
    }
    let page = template::test::Page {
        person: template::test::PagePerson { name: "King" },
    };
    assert_eq!(
        template::test::Vars {
            rows: vec![()],
            page
        }
        .render(),
        "[King]"
    );
}

#[test]
fn the_root_can_be_tested_as_well_as_written() {
    mod template {
        typed_handlebars::str!(
            "test",
            r#"{{#each rows}}[{{#if @root.title}}{{@root.title}}{{else}}-{{/if}}]{{/each}}"#
        );
    }
    assert_eq!(
        template::test::Vars {
            rows: vec![()],
            title: "Dub"
        }
        .render(),
        "[Dub]"
    );
    assert_eq!(
        template::test::Vars {
            rows: vec![()],
            title: ""
        }
        .render(),
        "[-]"
    );
}

/// The two constructs compose: `{{@root.rows.length}}` counts the top-level list from inside it.
#[test]
fn a_root_list_can_be_counted_from_inside_itself() {
    mod template {
        typed_handlebars::str!("test", r#"{{#each rows}}[{{@root.rows.length}}]{{/each}}"#);
    }
    assert_eq!(
        template::test::Vars {
            rows: vec![(), (), ()]
        }
        .render(),
        "[3][3][3]"
    );
}

/// One field, whichever way it is reached — `{{ title }}` at the top and `{{@root.title}}` inside a
/// loop are the same variable, and the constructor takes it once.
#[test]
fn a_root_reference_and_a_plain_one_name_the_same_field() {
    mod template {
        typed_handlebars::str!(
            "test",
            r#"{{ title }}{{#each rows}}[{{@root.title}}]{{/each}}"#
        );
    }
    assert_eq!(
        template::test::Vars {
            title: "Dub",
            rows: vec![()]
        }
        .render(),
        "Dub[Dub]"
    );
}

/// `{{#each @root.rows}}` and `{{#with @root.person}}` are legal in handlebars.js. They need no
/// separate machinery: `@root` is frame 0 by definition, which is what a block subject resolves to
/// anyway.
#[test]
fn the_root_can_be_a_block_subject() {
    mod iterated {
        typed_handlebars::str!("test", r#"{{#each @root.rows}}[{{ name }}]{{/each}}"#);
    }
    mod entered {
        typed_handlebars::str!(
            "test",
            r#"{{#each rows}}{{#with @root.person}}[{{ name }}]{{/with}}{{/each}}"#
        );
    }
    assert_eq!(
        iterated::test::Vars {
            rows: vec![iterated::test::RowsItem { name: "King" }]
        }
        .render(),
        "[King]"
    );
    assert_eq!(
        entered::test::Vars {
            rows: vec![()],
            person: entered::test::Person { name: "Tubby" }
        }
        .render(),
        "[Tubby]"
    );
}

/// A partial is spliced into its caller, so `@root` inside one means the including template's top
/// level — as it does in handlebars.js, where `@root` is the context the render started from.
#[test]
fn a_partial_sees_the_including_templates_root() {
    mod templates {
        typed_handlebars::directory!("tests/templates/root/");
    }
    assert_eq!(
        templates::page::Vars {
            rows: vec![templates::page::RowsItem { name: "King" }],
            title: "Dub",
        }
        .render(),
        "<li>King (Dub)</li>"
    );
}
