//! Naming a generated type in your own signatures.
//!
//! Most call sites let inference produce the type and never name it. Once an application has more
//! than one call site for the same template it usually wants the mapping from its own types in one
//! place — which means writing the type down.
//!
//! A type's parameters are exactly its values: one per field, in template order. The `Render`
//! markers that decide how a value is written live on `render` as method-level generics, so they
//! never appear here — not even for an `Option`, which is the case that used to force every marker
//! before it to be spelled too.

/// The application's own type, which the template knows nothing about.
struct Todo {
    id: i64,
    done: bool,
    title: String,
}

typed_handlebars::str!(
    "todo",
    //language=handlebars
    r#"<div id="todo-{{id}}">{{#if done}}[x]{{/if}}{{title}}</div>"#
);

/// The wiring lives in one function rather than at every call site. Spelling the return type is the
/// whole point, and it is exactly the three values the template uses.
fn todo_item(todo: &Todo) -> todo::Vars<i64, bool, &String> {
    todo::Vars {
        id: todo.id,
        done: todo.done,
        title: &todo.title,
    }
}

#[test]
fn a_helper_can_name_the_generated_type() {
    let todo = Todo {
        id: 7,
        done: true,
        title: "Write it down".into(),
    };
    assert_eq!(
        todo_item(&todo).render(),
        //language=html
        r#"<div id="todo-7">[x]Write it down</div>"#
    );
}

/// The same mapping as a conversion, which is the form an application reaches for once more than one
/// template wants it.
impl<'a> From<&'a Todo> for todo::Vars<i64, bool, &'a String> {
    fn from(todo: &'a Todo) -> Self {
        todo::Vars {
            id: todo.id,
            done: todo.done,
            title: &todo.title,
        }
    }
}

#[test]
fn the_generated_type_can_be_a_conversion_target() {
    let todo = Todo {
        id: 1,
        done: false,
        title: "Later".into(),
    };
    let item: todo::Vars<i64, bool, &String> = (&todo).into();
    assert_eq!(item.render(), r#"<div id="todo-1">Later</div>"#);
}

/// An item type is nameable on the same terms, so a list can be built by a helper and handed to the
/// template whole. This is the shape an application actually has: a `Vec` of its own records that
/// has to become a `Vec` of the template's.
#[test]
fn an_item_type_is_nameable_too() {
    typed_handlebars::str!(
        "list",
        //language=handlebars
        r#"<ul>{{#each todos}}<li>{{title}}</li>{{/each}}</ul>"#
    );

    fn rows(todos: &[Todo]) -> Vec<list::TodosItem<&String>> {
        todos
            .iter()
            .map(|todo| list::TodosItem { title: &todo.title })
            .collect()
    }

    let todos = vec![
        Todo {
            id: 1,
            done: false,
            title: "One".into(),
        },
        Todo {
            id: 2,
            done: true,
            title: "Two".into(),
        },
    ];
    assert_eq!(
        list::Vars {
            todos: rows(&todos)
        }
        .render(),
        //language=html
        "<ul><li>One</li><li>Two</li></ul>"
    );
}

/// A generated value can be held rather than rendered on the spot, which is what makes it different
/// from a rendered `String`: it is still the data, and it can be rendered more than once.
#[test]
fn a_generated_value_can_be_stored() {
    struct Page<'a> {
        rows: Vec<todo::Vars<i64, bool, &'a String>>,
    }

    let todos = [
        Todo {
            id: 1,
            done: true,
            title: "One".into(),
        },
        Todo {
            id: 2,
            done: false,
            title: "Two".into(),
        },
    ];
    let page = Page {
        rows: todos.iter().map(todo_item).collect(),
    };
    assert_eq!(
        page.rows.iter().map(|row| row.render()).collect::<String>(),
        r#"<div id="todo-1">[x]One</div><div id="todo-2">Two</div>"#
    );
}

/// A nested record declares its own parameters and its parent names it in a field, so the two have
/// to agree on the order.
#[test]
fn a_parent_and_a_nested_type_agree_on_order() {
    typed_handlebars::str!("rec", r#"{{person.name}}{{#if person.active}}[on]{{/if}}"#);

    assert_eq!(
        rec::Vars {
            person: rec::Person {
                name: "King",
                active: true
            }
        }
        .render(),
        "King[on]"
    );
    let person: rec::Person<&str, bool> = rec::Person {
        name: "King",
        active: false,
    };
    assert_eq!(rec::Vars { person }.render(), "King");
}

/// A list is where the parameters stop threading up: an item type is named only in an `AsRef`
/// bound, never in a field, so the parent keeps its container parameter opaque and the item's own
/// parameters are recovered on `render`. That is what keeps every type free of `PhantomData`, and
/// so writeable as a literal.
#[test]
fn nesting_two_deep_stays_nameable() {
    typed_handlebars::str!(
        "deep",
        //language=handlebars
        r#"{{#each rows}}{{label}}{{#each cells}}<{{value}}>{{/each}}{{/each}}"#
    );

    assert_eq!(
        deep::Vars {
            rows: vec![
                deep::RowsItem {
                    label: "a",
                    cells: vec![deep::RowsItemCellsItem { value: 1 }]
                },
                deep::RowsItem {
                    label: "b",
                    cells: vec![deep::RowsItemCellsItem { value: 2 }]
                },
            ]
        }
        .render(),
        "a<1>b<2>"
    );

    // Two parameters, not three: the label and the cell container. The cell's own value parameter
    // belongs to the bound, so it stays inside the container type rather than climbing out.
    let _: deep::RowsItem<&str, Vec<deep::RowsItemCellsItem<i32>>> = deep::RowsItem {
        label: "a",
        cells: vec![deep::RowsItemCellsItem { value: 1 }],
    };
}

/// An unset field falls back to a concrete empty type per parameter, and that list is parallel to
/// the parameters. If the two came apart, a builder that leaves everything unset would stop
/// compiling.
#[test]
fn unset_fields_still_resolve() {
    typed_handlebars::str!("unset", r#"{{a}}{{#each rows}}{{b}}{{/each}}{{c.d}}"#);

    assert_eq!(unset::builder().render(), "");
    assert_eq!(unset::builder().a("x").render(), "x");
}

/// `Option` does not take the `ViaDisplay` route, and used to be the one case where naming a type
/// meant naming its marker — and, a default only being elidable from the right, every marker before
/// it as well. With the markers on `render` instead, the signature is just the values.
#[test]
fn an_option_needs_no_marker() {
    typed_handlebars::str!("maybe", r#"[{{a}}][{{b}}]"#);

    fn pair(b: Option<u32>) -> maybe::Vars<&'static str, Option<u32>> {
        maybe::Vars { a: "x", b }
    }

    assert_eq!(pair(Some(1)).render(), "[x][1]");
    // `None` writes nothing, as null does in handlebars.js.
    assert_eq!(pair(None).render(), "[x][]");
}
