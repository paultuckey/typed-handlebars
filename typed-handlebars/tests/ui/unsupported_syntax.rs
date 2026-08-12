// Handlebars this crate does not support yet must say so by name, rather than reaching the compiler
// as invalid Rust.
mod partial {
    typed_handlebars::str!("page", "A{{> other}}B");
}

mod else_if {
    typed_handlebars::str!("page", "{{#if a}}1{{else if b}}2{{else}}3{{/if}}");
}

mod unknown_helper {
    typed_handlebars::str!("page", "{{#wat x}}y{{/wat}}");
}

fn main() {}
