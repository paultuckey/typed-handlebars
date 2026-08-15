// `{{ rows.[0] }}` is real Handlebars, and not supported here. Left alone it would quietly become
// a record named `Rows` with a field called `[0]` — a type that compiles and renders the wrong
// thing, which is the one outcome this crate promises never to produce.
mod template {
    typed_handlebars::str!("page", r#"<h1>{{ rows.[0] }}</h1>"#);
}

fn main() {}
