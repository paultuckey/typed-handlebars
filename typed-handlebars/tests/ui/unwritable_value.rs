// A template writes anything that displays, and an `Option` of it. Anything else is a mistake at
// the call site, and the error should say so in template terms rather than naming the `Render`
// marker that inference would otherwise have picked.
mod template {
    typed_handlebars::str!("page", r#"<h1>{{ title }}</h1>"#);
}

struct Heading {
    _text: String,
}

// Not `.render()` — see the note in `tests/ui.rs`: a method-resolution error quotes `Display` out
// of the standard library, which only renders where `rust-src` is installed.
fn main() {
    let _ = template::page(Heading {
        _text: "Dub".into(),
    });
}
