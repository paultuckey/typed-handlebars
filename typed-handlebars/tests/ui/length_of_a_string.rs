// `.length` counts a list. A `String` has one in JS, but JS counts UTF-16 code units where Rust
// counts bytes or `char`s — all three disagree on the same text — so it is refused rather than
// answered with a quietly different number.
mod template {
    typed_handlebars::str!("page", r#"<h1>{{ title.length }}</h1>"#);
}

// Deliberately *not* `.render()`. Calling it adds an `E0599` "method exists but its trait bounds
// were not satisfied", which quotes `String`'s definition out of the standard library — and rustc
// can only do that when `rust-src` is installed, which it is on a developer's rustup and is not on
// a CI runner. The bound is already reported at the constructor, which is the diagnostic worth
// pinning; see the note in `tests/ui.rs`.
fn main() {
    let _ = template::page(String::from("Dub"));
}
