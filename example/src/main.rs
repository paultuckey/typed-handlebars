mod templates {
    dry_handlebars::directory!("templates/");
    dry_handlebars::file!("template/button2.hbs");
    //language=html
    dry_handlebars::str!(
        "hello_first_last",
        r#"
        <p>Hello {{firstname}} {{lastname}}</p>
    "#
    );
}

fn main() {
    let html = templates::button(42, "My Todo").render();
    println!("{}", html);

    let html2 = templates::button2(43, "Single File Todo").render();
    println!("{}", html2);

    let html3 = templates::hello_first_last("King", "Tubby").render();
    println!("{}", html3);

    // todo_list.hbs uses {{#each}}, so the macro generates the item types from the template —
    // nothing here declares a type or implements a trait.
    let html4 = templates::todo_list(
        "Today",
        vec![
            templates::todo_list::TodosItem::new(
                1,
                "Buy milk",
                templates::todo_list::TodosItemOwner::new("King"),
            ),
            templates::todo_list::TodosItem::new(
                2,
                "Write docs",
                templates::todo_list::TodosItemOwner::new("Tubby"),
            ),
        ],
    )
    .render();
    println!("{}", html4);

    // The builder is optional. It names each variable, and anything left unset renders empty —
    // here there are no todos, so the loop produces nothing.
    let html5 = templates::todo_list::Builder::new()
        .list_name("Nothing to do")
        .render();
    println!("{}", html5);
}
