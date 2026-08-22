// A helper resolves against the frame, and `register_helper!` is what names it. Forget the line
// and there is nothing to resolve on — so the error has to name what it went looking for, since
// that name is the only thread back to the macro that supplies it.
//
// On its own in a file: rustc lists any `Frame` that exists elsewhere in the crate as a suggestion,
// so a sibling module would make this expectation churn every time one was added.
mod templates {
    typed_handlebars::str!("page", r#"{{ t "Save" }}"#);
}

fn main() {}
