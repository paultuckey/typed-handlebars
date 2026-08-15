// `.length` counts a list. A `String` has one in JS, but JS counts UTF-16 code units where Rust
// counts bytes or `char`s — all three disagree on the same text — so it is refused rather than
// answered with a quietly different number.
mod template {
    typed_handlebars::str!("page", r#"<h1>{{ title.length }}</h1>"#);
}

fn main() {
    let _ = template::page(String::from("Dub")).render();
}
