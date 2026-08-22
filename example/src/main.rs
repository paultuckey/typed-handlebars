mod templates {
    typed_handlebars::directory!("templates/");
    typed_handlebars::file!("template/button2.hbs");
    //language=html
    typed_handlebars::str!(
        "hello_first_last",
        r#"
        <p>Hello {{firstname}} {{lastname}}</p>
    "#
    );
    // Basic helpers are supported
    typed_handlebars::register_helper!(crate::Frame);
    //language=html
    typed_handlebars::str!(
        "hello_money",
        r#"
        <p>{{ hello "world" }}</p>
        <p>{{ money total }}</p>
    "#
    );
}

fn main() {
    // Every variable the template uses, named. Miss one and it is a compile error saying which.
    let html = templates::button::Vars {
        btn_id: 42,
        btn_name: "My Todo",
    }
    .render();
    println!("{}", html);

    let html2 = templates::button2::Vars {
        btn_id: 43,
        btn_name: "Single File Todo",
    }
    .render();
    println!("{}", html2);

    let html3 = templates::hello_first_last::Vars {
        firstname: "King",
        lastname: "Tubby",
    }
    .render();
    println!("{}", html3);

    // todo_list.hbs uses {{#each}}, so the macro generates the item types from the template —
    // nothing here declares a type or implements a trait.
    let html4 = templates::todo_list::Vars {
        list_name: "Today",
        todos: vec![
            templates::todo_list::TodosItem {
                todo_id: 1,
                title: "Buy milk",
                owner: templates::todo_list::TodosItemOwner { name: "King" },
            },
            templates::todo_list::TodosItem {
                todo_id: 2,
                title: "Write docs",
                owner: templates::todo_list::TodosItemOwner { name: "Tubby" },
            },
        ],
    }
    .render();
    println!("{}", html4);

    // The builder is for when you do not have every variable. Anything left unset renders empty —
    // here there are no todos, so the loop produces nothing.
    let html5 = templates::todo_list::builder()
        .list_name("Nothing to do")
        .render();
    println!("{}", html5);

    // A template that calls a helper asks for the frame as well as the data. The ones above call
    // none, so their `render()` is unchanged.
    let frame = Frame {
        greeting: "Hello",
        currency: "$",
    };
    let html6 = templates::hello_money::Vars { total: 4200 }.render(&frame);
    println!("{}", html6);
}

// Frame is passed at render time beside the context.
pub struct Frame {
    greeting: &'static str,
    currency: &'static str,
}

impl Frame {
    pub fn hello(&self, key: &str) -> String {
        format!("{} {}", self.greeting, key)
    }

    pub fn money(&self, amount: &str) -> String {
        format!("${}{}", self.currency, amount)
    }
}
