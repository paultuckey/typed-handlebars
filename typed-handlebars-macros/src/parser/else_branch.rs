//! Reads an `{{else …}}` expression.
//!
//! Both halves of the parser look at `else`: [`context`](super::context) to work out which
//! variables a template needs, and [`compiler`](super::compiler) to emit the branch. They have to
//! agree about what an `else` expression means — including that `{{ else }}` with spaces in it is
//! still an `else` and not a variable called `else` — so both read it through here.
//!
//! # What Handlebars means by `{{else if}}`
//!
//! It is sugar: `{{#if a}}A{{else if b}}B{{/if}}` behaves as
//! `{{#if a}}A{{else}}{{#if b}}B{{/if}}{{/if}}`, with the one `{{/if}}` closing both. That maps
//! exactly onto a Rust `else if` chain, which is why the whole chain still needs exactly one
//! closing brace.
//!
//! Two details, both checked against handlebars.js rather than assumed:
//!
//! - **The chained helper decides the sense of the test, not the block it sits in.**
//!   `{{#unless a}}U{{else if b}}B{{/unless}}` tests `b` for *truth*. `{{else unless b}}` is
//!   equally legal and tests it for falsity.
//! - **Anything can be chained in handlebars.js**, including `{{else each xs}}`. Those open a
//!   scope that the outer block's close would have to close too, so they are rejected by name —
//!   see [`ElseBranch::UnsupportedHelper`].

/// What an `{{else …}}` expression asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum ElseBranch<'a> {
    /// `{{else}}` — the alternative branch of whichever block it sits in.
    Plain,
    /// `{{else if cond}}`, or `{{else unless cond}}` with `negated` set.
    ///
    /// `condition` is the text after the helper, empty if there wasn't any.
    Chained { negated: bool, condition: &'a str },
    /// `{{else each xs}}`, `{{else with p}}` and anything else chained onto an `else`.
    UnsupportedHelper(&'a str),
}

impl ElseBranch<'_> {
    /// How to spell this branch in a message, without the braces.
    pub fn keyword(&self) -> &'static str {
        match self {
            ElseBranch::Plain => "else",
            ElseBranch::Chained { negated: false, .. } => "else if",
            ElseBranch::Chained { negated: true, .. } => "else unless",
            ElseBranch::UnsupportedHelper(_) => "else",
        }
    }
}

/// Reads an expression's content as an `{{else …}}` form, or `None` when it is not one.
pub fn classify(content: &str) -> Option<ElseBranch<'_>> {
    let rest = content.trim().strip_prefix("else")?;
    if rest.is_empty() {
        return Some(ElseBranch::Plain);
    }
    // `{{elsewhere}}` starts with `else` and is a variable. Only a word boundary makes it a branch.
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    let (helper, condition) = match rest.find(char::is_whitespace) {
        Some(end) => (&rest[..end], rest[end..].trim()),
        None => (rest, ""),
    };
    Some(match helper {
        "if" => ElseBranch::Chained {
            negated: false,
            condition,
        },
        "unless" => ElseBranch::Chained {
            negated: true,
            condition,
        },
        other => ElseBranch::UnsupportedHelper(other),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_else_is_plain_however_it_is_spaced() {
        assert_eq!(classify("else"), Some(ElseBranch::Plain));
        assert_eq!(classify("  else  "), Some(ElseBranch::Plain));
    }

    #[test]
    fn else_if_carries_its_condition() {
        assert_eq!(
            classify("else if b"),
            Some(ElseBranch::Chained {
                negated: false,
                condition: "b"
            })
        );
        assert_eq!(
            classify("else   if   person.name  "),
            Some(ElseBranch::Chained {
                negated: false,
                condition: "person.name"
            })
        );
    }

    /// The chained helper decides the sense of the test, so `unless` negates even inside an
    /// `{{#if}}`.
    #[test]
    fn else_unless_is_negated() {
        assert_eq!(
            classify("else unless b"),
            Some(ElseBranch::Chained {
                negated: true,
                condition: "b"
            })
        );
    }

    #[test]
    fn a_chain_with_nothing_to_test_keeps_its_shape() {
        assert_eq!(
            classify("else if"),
            Some(ElseBranch::Chained {
                negated: false,
                condition: ""
            })
        );
    }

    #[test]
    fn other_helpers_are_named_rather_than_guessed_at() {
        assert_eq!(
            classify("else each xs"),
            Some(ElseBranch::UnsupportedHelper("each"))
        );
        assert_eq!(
            classify("else with p"),
            Some(ElseBranch::UnsupportedHelper("with"))
        );
    }

    /// The whole point of the word-boundary check: these are variables.
    #[test]
    fn a_variable_that_merely_starts_with_else_is_not_a_branch() {
        assert_eq!(classify("elsewhere"), None);
        assert_eq!(classify("else_if"), None);
        assert_eq!(classify("elsewhere.town"), None);
        assert_eq!(classify(""), None);
        assert_eq!(classify("name"), None);
    }
}
