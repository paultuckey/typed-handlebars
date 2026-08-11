//! Compile-fail tests pinning the diagnostics a developer sees when they get the wiring wrong.
//!
//! Error quality is a feature here, not a nicety: the whole point of the builder is that a missing
//! or renamed template variable is caught at compile time *and* says what to do about it. These
//! tests exist so that stays true.
//!
//! The expected output lives in `tests/ui/*.stderr`. After an intentional change, regenerate it
//! with `TRYBUILD=overwrite cargo test -p dry-handlebars --test ui` and read the diff.

#[test]
fn wiring_mistakes_are_reported_clearly() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
