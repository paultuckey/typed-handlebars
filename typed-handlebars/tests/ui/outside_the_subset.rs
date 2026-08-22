// Handlebars that this crate does not implement must say so by name. None of these may reach the
// Rust compiler as a type error — the person who wrote the template cannot act on one of those.
mod sub_expression {
    typed_handlebars::str!("page", "{{(lookup a 1)}}");
}

mod private_variable {
    typed_handlebars::str!("page", "{{#each rows}}{{@key}}{{/each}}");
}

// A private that exists but has no loop to come from. `{{#if}}` is transparent to an `@…` lookup,
// so the second one reports the missing loop rather than blaming the `{{#if}}`.
mod private_outside_a_loop {
    typed_handlebars::str!("page", "{{@index}}");
}

mod private_with_only_a_conditional_around_it {
    typed_handlebars::str!("page", "{{#if a}}{{@index}}{{/if}}");
}

mod private_reaching_past_the_outermost_loop {
    typed_handlebars::str!("page", "{{#each rows}}{{@../index}}{{/each}}");
}

fn main() {}
