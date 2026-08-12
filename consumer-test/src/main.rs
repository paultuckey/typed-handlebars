//! Builds the README's headline example verbatim, as a stranger's crate would.
//!
//! This is the project goal expressed as a test: a consumer wires up their data and writes nothing
//! else. The lints below are the teeth — every one of them fires on generated code if the macro
//! leaks a `use`, an undocumented item, or a path that only resolves inside this crate.
//!
//! There is deliberately no `use`, no `#[allow]`, and no trait `impl` anywhere in this file. If
//! adding one ever becomes necessary to make this compile, the goal has regressed.
#![deny(missing_docs)]
#![deny(unused)]
#![deny(unreachable_pub)]
#![deny(missing_debug_implementations)]
#![deny(clippy::pedantic)]

mod templates {
    typed_handlebars::directory!("templates/");
}

/// The README example, unchanged.
fn get_html() -> String {
    // templates::button is automatically generated
    templates::button(42, "Save").render()
}

/// Checks the generated code against what the template says it should produce.
fn main() {
    let html = get_html();
    assert_eq!(
        html,
        "<button id=\"btn42\" class=\"btn btn-light\">\n    Save\n</button>\n",
        "positional call"
    );

    // The builder is the other documented entry point, so it is covered here too.
    let built = templates::button::Builder::new()
        .btn_name("Save")
        .btn_id(42)
        .render();
    assert_eq!(built, html, "builder matches the positional call");

    // Escaping is a consumer-visible promise, so assert it from out here rather than only inside
    // the crate's own tests.
    let escaped = templates::button(1, "<script>&").render();
    assert!(
        escaped.contains("&lt;script&gt;&amp;"),
        "`{{{{ }}}}` escapes: {escaped}"
    );

    println!("consumer-test: generated code compiles and renders correctly under deny lints");
}
