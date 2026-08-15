//! Compile-fail tests pinning the diagnostics a developer sees when they get the wiring wrong.
//!
//! Error quality is a feature here, not a nicety: the whole point of the builder is that a missing
//! or renamed template variable is caught at compile time *and* says what to do about it. These
//! tests exist so that stays true.
//!
//! The expected output lives in `tests/ui/*.stderr`. After an intentional change, regenerate it
//! with `TRYBUILD=overwrite cargo test -p typed-handlebars --test ui` and read the diff.
//!
//! # Expectations must not quote the standard library
//!
//! Some diagnostics — `E0599`'s "method exists but its trait bounds were not satisfied" is the one
//! that bit us — quote the definition of a std type or trait, rendered from the standard library's
//! *source*. rustc can only do that where the `rust-src` component is installed. It is on a
//! developer's rustup, and it is not on a GitHub runner, so an expectation containing one passes
//! locally and fails in CI with a diff nobody can reproduce.
//!
//! [`expectations_do_not_depend_on_rust_src`] makes that reproducible: it fails on any machine, at
//! the moment the expectation is blessed. The fix is to write the case so the diagnostic never
//! reaches for std — usually by dropping a trailing method call, since the bound is already
//! reported against the constructor.

use std::fs;

#[test]
fn wiring_mistakes_are_reported_clearly() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}

/// Guards the rule in this file's docs: no expectation may quote the standard library's source.
///
/// `$RUST` is trybuild's placeholder for the sysroot, so its presence is exactly the signal that a
/// diagnostic reached into std and that the case will behave differently without `rust-src`.
#[test]
fn expectations_do_not_depend_on_rust_src() {
    let mut offenders = Vec::new();
    for entry in fs::read_dir("tests/ui").expect("the ui directory is beside this file") {
        let path = entry.expect("readable directory entry").path();
        if path
            .extension()
            .is_some_and(|extension| extension == "stderr")
            && fs::read_to_string(&path)
                .expect("readable expectation")
                .contains("$RUST")
        {
            offenders.push(path.display().to_string());
        }
    }
    offenders.sort();
    assert!(
        offenders.is_empty(),
        "these expectations quote the standard library's source, so they pass only where \
         `rust-src` is installed and fail in CI: {}.\nRewrite the case so the diagnostic stays \
         within the template and this crate — see the note at the top of tests/ui.rs.",
        offenders.join(", ")
    );
}
