//! Reads the parts of a path that are not field names.
//!
//! Both halves of the parser walk paths: [`context`](super::context) to work out what data a
//! template needs, and [`compiler`](super::compiler) to emit the access. They have to agree about
//! which parts of a path are *not* a field — a trailing `.length` counts a list, and `@root.`
//! reaches the top-level context — so both read them through here.
//!
//! [`else_branch`](super::else_branch) exists for the same reason. The two walks drifted over
//! `{{ else }}` once, visibly, and a shared classifier is what stopped it; any construct read by
//! both wants the same treatment.

/// Splits a trailing `.length` off a path, giving back what is being counted.
///
/// `{{ rows.length }}` is how many items `rows` holds, as it is in handlebars.js, where `length` is
/// an ordinary property lookup that lands on the one JS arrays carry. A bare `{{ length }}` is an
/// ordinary variable of that name, so only a `.length` with something in front of it counts.
///
/// ```ignore
/// assert_eq!(counted("rows.length"), Some("rows"));
/// assert_eq!(counted("page.rows.length"), Some("page.rows"));
/// assert_eq!(counted("length"), None);
/// assert_eq!(counted("rows"), None);
/// ```
pub fn counted(path: &str) -> Option<&str> {
    match path.strip_suffix(".length") {
        Some(subject) if !subject.is_empty() => Some(subject),
        _ => None,
    }
}

/// The path beneath `{{@root.…}}`, if this `@…` variable names the top-level context.
///
/// `@root` is **absolute**, which is what separates it from `@index`, `@first` and `@last`: those
/// are loop state and step outwards with `../`, while `@root` names the top of the template from
/// any depth. Checked against handlebars.js: `{{@../root.title}}` reads the same value as
/// `{{@root.title}}`, so the prefix is stripped and ignored rather than walked.
///
/// The name arrives with its `@` already removed, and bare `root` gives `None` — there is nothing
/// useful to write for the whole context, which handlebars.js renders as `[object Object]`.
///
/// ```ignore
/// assert_eq!(under_root("root.title"), Some("title"));
/// assert_eq!(under_root("../root.title"), Some("title"));
/// assert_eq!(under_root("root"), None);
/// assert_eq!(under_root("index"), None);
/// ```
pub fn under_root(name: &str) -> Option<&str> {
    match name.trim_start_matches("../").strip_prefix("root.") {
        Some(path) if !path.is_empty() => Some(path),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_length_needs_something_to_count() {
        assert_eq!(counted("rows.length"), Some("rows"));
        assert_eq!(counted("page.rows.length"), Some("page.rows"));
        // A variable of that name, not a count of anything.
        assert_eq!(counted("length"), None);
        assert_eq!(counted(".length"), None);
        assert_eq!(counted("rows"), None);
        // Only the whole final segment counts.
        assert_eq!(counted("rows.lengths"), None);
        assert_eq!(counted("rowslength"), None);
    }

    #[test]
    fn root_is_absolute() {
        assert_eq!(under_root("root.title"), Some("title"));
        assert_eq!(under_root("root.person.name"), Some("person.name"));
        // `../` does not walk out of `@root`; it means the same thing.
        assert_eq!(under_root("../root.title"), Some("title"));
        assert_eq!(under_root("../../root.title"), Some("title"));
    }

    #[test]
    fn other_privates_are_not_root() {
        assert_eq!(under_root("index"), None);
        assert_eq!(under_root("../first"), None);
        // The whole context has nothing to write, so it is not a path.
        assert_eq!(under_root("root"), None);
        assert_eq!(under_root("root."), None);
        // A private that merely starts with the letters.
        assert_eq!(under_root("rooted.x"), None);
    }

    #[test]
    fn the_two_compose_as_they_do_in_a_template() {
        // `{{@root.rows.length}}` — the top-level `rows`, counted.
        let path = under_root("root.rows.length").expect("names the root");
        assert_eq!(counted(path), Some("rows"));
    }
}
