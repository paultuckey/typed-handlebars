// `.length` counts a list. A `String` has one in JS, but JS counts UTF-16 code units where Rust
// counts bytes or `char`s — all three disagree on the same text — so it is refused rather than
// answered with a quietly different number.
mod template {
    typed_handlebars::str!("page", r#"<h1>{{ title.length }}</h1>"#);
}

// The bound is on `render` rather than on the type, so rendering is what reports it.
fn main() {
    let _ = template::page::Vars {
        title: String::from("Dub"),
    }
    .render();
}
