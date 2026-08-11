// Handlebars that this crate does not implement must say so by name. None of these may reach the
// Rust compiler as a type error — the person who wrote the template cannot act on one of those.
mod custom_helper {
    dry_handlebars::str!("page", "{{myhelper x}}");
}

mod sub_expression {
    dry_handlebars::str!("page", "{{(lookup a 1)}}");
}

mod private_variable {
    dry_handlebars::str!("page", "{{#each rows}}{{@key}}{{/each}}");
}

fn main() {}
