//! Naming a generated type in your own signatures.
//!
//! A written value is bound by `Render<K>`, and `K` is a marker filled in by inference. It has to be
//! a parameter of the generated type, because the type's `where` clause names it — so it lands in
//! the public signature of a type nobody was ever meant to spell.
//!
//! Markers are declared last and default to `ViaDisplay`, so the common case elides them entirely:
//! `Template<i64, bool, &String>` rather than `Template<i64, ViaDisplay, bool, &String, ViaDisplay>`.
//! That is what lets an application put the mapping from its own types to a template's in one place,
//! instead of repeating it at every call site.

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
/// whole point: before markers were defaulted, this signature could not be written at all.
fn todo_item(todo: &Todo) -> todo::Template<i64, bool, &String> {
    todo::Template::new(todo.id, todo.done, &todo.title)
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
impl<'a> From<&'a Todo> for todo::Template<i64, bool, &'a String> {
    fn from(todo: &'a Todo) -> Self {
        todo::Template::new(todo.id, todo.done, &todo.title)
    }
}

#[test]
fn the_generated_type_can_be_a_conversion_target() {
    let todo = Todo {
        id: 1,
        done: false,
        title: "Later".into(),
    };
    let item: todo::Template<i64, bool, &String> = (&todo).into();
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
            .map(|todo| list::TodosItem::new(&todo.title))
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
        list(rows(&todos)).render(),
        //language=html
        "<ul><li>One</li><li>Two</li></ul>"
    );
}

/// A generated value can be held rather than rendered on the spot — which the `impl Display` escape
/// hatch could not do, because erasing the type means the value can only be written, never handled.
#[test]
fn a_generated_value_can_be_stored() {
    struct Page<'a> {
        rows: Vec<todo::Template<i64, bool, &'a String>>,
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

/// A nested type declares its own parameters, and its parent names it — so the two have to agree on
/// the order after markers move to the back.
///
/// The case that catches a mistake is a record whose *rendered* field comes before a field that is
/// only tested: in template order the marker sits in the middle, so parent and child would disagree
/// if either emitted the parameters as they were minted rather than as declared.
#[test]
fn a_parent_and_a_nested_type_agree_on_order() {
    typed_handlebars::str!("rec", r#"{{person.name}}{{#if person.active}}[on]{{/if}}"#);

    assert_eq!(rec(rec::Person::new("King", true)).render(), "King[on]");
    let person: rec::Person<&str, bool> = rec::Person::new("King", false);
    assert_eq!(rec(person).render(), "King");
}

/// The same agreement two levels down, where a list's item type is named only in an `AsRef` bound
/// and its parameters are threaded up into the parent.
#[test]
fn nesting_two_deep_stays_nameable() {
    typed_handlebars::str!(
        "deep",
        //language=handlebars
        r#"{{#each rows}}{{label}}{{#each cells}}<{{value}}>{{/each}}{{/each}}"#
    );

    assert_eq!(
        deep(vec![
            deep::RowsItem::new("a", vec![deep::RowsItemCellsItem::new(1)]),
            deep::RowsItem::new("b", vec![deep::RowsItemCellsItem::new(2)]),
        ])
        .render(),
        "a<1>b<2>"
    );

    // A parent carries its whole subtree's value parameters — the label, the cell value and the
    // cell container — but still none of the four markers.
    let _: deep::RowsItem<&str, i32, Vec<deep::RowsItemCellsItem<i32>>> =
        deep::RowsItem::new("a", vec![deep::RowsItemCellsItem::new(1)]);
}

/// An unset field falls back to a concrete empty type per parameter, and that list is parallel to
/// the parameters — so it is permuted along with them. If the two came apart, a builder that leaves
/// everything unset would stop compiling.
#[test]
fn unset_fields_still_resolve() {
    typed_handlebars::str!("unset", r#"{{a}}{{#each rows}}{{b}}{{/each}}{{c.d}}"#);

    assert_eq!(unset::Builder::new().render(), "");
    assert_eq!(unset::Builder::new().a("x").render(), "x");
}

/// `Option` is the one value that does not take the `ViaDisplay` route, so naming a type that holds
/// one means naming its marker — and, because a default can only be elided from the right, every
/// marker before it as well.
///
/// Worth pinning: it is the case where the defaults do not hide the markers, and the reason the
/// win is "most templates" rather than "all of them".
#[test]
fn an_option_names_its_marker() {
    typed_handlebars::str!("maybe", r#"[{{a}}][{{b}}]"#);
    use typed_handlebars::{ViaDisplay, ViaOption};

    // `a` renders through `Display` and `b` through `Option`, so `a`'s marker has to be spelled
    // even though it is exactly the default.
    fn pair(b: Option<u32>) -> maybe::Template<&'static str, Option<u32>, ViaDisplay, ViaOption> {
        maybe::Template::new("x", b)
    }

    assert_eq!(pair(Some(1)).render(), "[x][1]");
    // `None` writes nothing, as null does in handlebars.js.
    assert_eq!(pair(None).render(), "[x][]");
}
