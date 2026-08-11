//! Generated code must compile whatever the call site has in scope, and under whatever lints it
//! enables.
//!
//! The developer's side of the bargain is wiring, not accommodating the macro: no `use`, no
//! `#[allow]`, no avoiding names the expansion happens to want.

// Lints a consumer might reasonably enable crate-wide. Generated code has to satisfy them without
// the developer adding an `#[allow]` — needing one would be the macro imposing on the call site.
#![deny(missing_docs)]
#![deny(non_camel_case_types)]
#![deny(missing_debug_implementations)]

/// Shadows of every prelude item and macro a template expansion could reach for.
///
/// These are scaffolding rather than the thing under test, so the lints above are relaxed here and
/// nowhere else.
mod shadows {
    #![allow(missing_docs, missing_debug_implementations, dead_code, unused_macros)]

    // Types from the prelude.
    pub struct String;
    pub struct Result;
    pub struct Option;
    pub struct Vec;
    pub trait Display {}
    pub trait Write {}
    pub trait AsRef {}
    pub trait Default {}

    // `std` and `core` themselves, so a bare `std::fmt::Write` would resolve here.
    pub mod std {}
    pub mod core {}

    // Prelude macros. A template that reached for these instead of `::core::…` would fail loudly
    // rather than silently picking up the call site's version.
    macro_rules! write {
        ($($tt:tt)*) => {
            compile_error!("generated code used the call site's `write!`")
        };
    }
    macro_rules! include_bytes {
        ($($tt:tt)*) => {
            compile_error!("generated code used the call site's `include_bytes!`")
        };
    }
    pub(crate) use {include_bytes, write};

    // And the constructors.
    #[allow(non_upper_case_globals)]
    pub const Ok: () = ();
    #[allow(non_upper_case_globals)]
    pub const Some: () = ();
}

/// Templates expanded with every shadow above in scope, and subject to the crate's lints.
pub mod templates {
    #[allow(unused_imports)]
    use super::shadows::*;
    #[allow(unused_imports)]
    use super::shadows::{include_bytes, write};

    dry_handlebars::directory!("tests/templates/");
}

#[test]
fn generated_code_needs_nothing_from_the_call_site() {
    assert_eq!(
        templates::page("Dub", vec![templates::page_rows_item::new(1, "King")]).render(),
        "<h1>Dub</h1><ul><li id=\"r1\">King</li></ul>"
    );
}
