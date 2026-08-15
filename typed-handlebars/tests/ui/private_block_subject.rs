// `{{#each @root.rows}}` works, because `@root` names data. Every other `@…` is loop state, and
// iterating one is refused by name — left alone it would quietly become a record called `index`.
mod template {
    typed_handlebars::str!(
        "page",
        r#"{{#each rows}}{{#each @index}}<li>{{ name }}</li>{{/each}}{{/each}}"#
    );
}

fn main() {}
