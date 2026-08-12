// An unclosed block should name the block and where it opened, not surface as a proc-macro panic.
mod template {
    typed_handlebars::str!("page", "<ul>\n  {{#each rows}}\n    <li>{{ name }}</li>\n</ul>");
}

fn main() {}
