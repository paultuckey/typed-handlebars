// Handlebars this crate does not support yet must say so by name, rather than reaching the compiler
// as invalid Rust.
mod partial {
    typed_handlebars::str!("page", "A{{> other}}B");
}

// `{{else if}}` itself is supported now — see `tests/else_if.rs`. What is left is the chains that
// cannot compile to a Rust `else if`: one onto a block whose `{{else}}` is not a plain alternative,
// and one onto a helper that would open a scope the enclosing close cannot also close.
mod else_if_on_each {
    typed_handlebars::str!("page", "{{#each xs}}1{{else if b}}2{{/each}}");
}

mod else_if_on_with {
    typed_handlebars::str!("page", "{{#with p}}1{{else if b}}2{{/with}}");
}

mod else_opening_a_scope {
    typed_handlebars::str!("page", "{{#if a}}1{{else each xs}}2{{/if}}");
}

mod else_if_with_nothing_to_test {
    typed_handlebars::str!("page", "{{#if a}}1{{else if}}2{{/if}}");
}

mod unknown_helper {
    typed_handlebars::str!("page", "{{#wat x}}y{{/wat}}");
}

fn main() {}
