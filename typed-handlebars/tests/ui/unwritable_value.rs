// A template writes anything that displays, and an `Option` of it. Anything else is a mistake at
// the call site, and the error should say so in template terms rather than naming the `Render`
// marker that inference would otherwise have picked.
mod template {
    typed_handlebars::str!("page", r#"<h1>{{ title }}</h1>"#);
}

struct Heading {
    _text: String,
}

// The bounds live on `render`, so it takes rendering to reach them: a `Vars` holding anything at
// all is a well-formed value, and only writing it out asks whether it can be written.
fn main() {
    let _ = template::page::Vars {
        title: Heading {
            _text: "Dub".into(),
        },
    }
    .render();
}
