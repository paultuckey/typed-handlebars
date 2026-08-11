// Variables the macro types itself fall back to empty when a builder leaves them out. One whose
// type was declared in Rust has no empty to fall back on, so it has to be set.
struct Author {
    first_name: String,
}

mod template {
    dry_handlebars::str!(
        "page",
        r#"{{#each authors}}<p>{{first_name}}</p>{{/each}}"#,
        ("authors", Vec<super::Author>)
    );
}

fn main() {
    let _ = template::page_builder::new().render();
}
