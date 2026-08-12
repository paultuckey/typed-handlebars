//! Infers the shape of the data a template needs, from the template alone.
//!
//! The template is the master: an `.hbs` file is written by someone who need not know any Rust, so
//! it cannot carry Rust type annotations. Everything the generated code needs about the caller's
//! data therefore has to be read out of the Handlebars itself.
//!
//! A template like
//!
//! ```handlebars
//! <h1>{{ title }}</h1>
//! {{#each rows}}<li>{{ name }} ({{ user.email }})</li>{{/each}}
//! ```
//!
//! states its own contract: a `title` that gets displayed, a sequence `rows`, and — inside each
//! row — a `name` that gets displayed and a `user` with an `email`. [`build`] turns that into a
//! [`Context`] tree, which code generation then turns into Rust types.
//!
//! # Scoping
//!
//! Scope resolution mirrors [`crate::parser::compiler::Compile::resolve_var`] exactly, because the
//! two have to agree about which struct a field lands on:
//!
//! - `{{#each rows}}` with no alias makes the item the implicit context, so a bare `{{ name }}`
//!   inside is `item.name` and `{{ this }}` is the item itself.
//! - `{{#each rows as |row|}}` names the item instead, so `{{ row.name }}` is `item.name` while a
//!   bare `{{ name }}` falls through to the enclosing scope.
//! - `{{ ../name }}` walks one scope outwards per `../`.

use crate::parser::error::{ParseError, Result};
use crate::parser::expression::{Expression, ExpressionType};
use crate::parser::expression_tokenizer::{Token, TokenType};

/// What a template does with a variable, and hence what type it needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    /// Used only as a value — `{{ name }}`. Needs to implement `Display`.
    Leaf,
    /// Iterated over — `{{#each rows}}`. The [`Context`] describes one item.
    Sequence(Context),
    /// Has sub-fields — `{{ user.email }}` or `{{#with user}}`.
    Object(Context),
}

/// One variable a template uses, and everything the template says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// The name exactly as the template spells it.
    pub name: String,
    pub kind: FieldKind,
    /// Written out somewhere — `{{ name }}` or `{{{ name }}}`.
    pub used_as_value: bool,
    /// Tested somewhere — `{{#if name}}` or `{{#unless name}}`.
    pub used_as_condition: bool,
}

impl Field {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            kind: FieldKind::Leaf,
            used_as_value: false,
            used_as_condition: false,
        }
    }
}

/// The data one scope of a template needs: the root context, or one item of an `{{#each}}`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Context {
    /// Fields in the order the template first mentions them.
    pub fields: Vec<Field>,
    /// The scope itself is written out — `{{ this }}` inside an `{{#each}}` over scalars.
    pub used_as_value: bool,
}

impl Context {
    /// True when nothing in the template needs this scope to have any structure — an `{{#each}}`
    /// over plain values rather than over records.
    pub fn is_scalar(&self) -> bool {
        self.fields.is_empty()
    }

    fn index_of(&mut self, name: &str) -> usize {
        match self.fields.iter().position(|f| f.name == name) {
            Some(i) => i,
            None => {
                self.fields.push(Field::new(name));
                self.fields.len() - 1
            }
        }
    }

    /// Returns the inner context of `name`, treating it as a record.
    ///
    /// Seeing `{{ user.email }}` is what tells us `user` is a record rather than a value, so a
    /// field the template has so far only mentioned bare gets promoted here.
    fn object_at(&mut self, name: &str) -> Result<&mut Context> {
        let index = self.index_of(name);
        let field = &mut self.fields[index];
        if let FieldKind::Leaf = field.kind {
            field.kind = FieldKind::Object(Context::default());
        }
        match &mut field.kind {
            FieldKind::Object(inner) => Ok(inner),
            FieldKind::Sequence(_) => Err(ParseError::general(&format!(
                "`{}` is used both as a list and as a record — a `{{{{#each {}}}}}` block and a \
                 `{{{{ {}.… }}}}` reference cannot both be right",
                name, name, name
            ))),
            FieldKind::Leaf => unreachable!("promoted to Object above"),
        }
    }

    /// Creates the field at `path` without recording any use of it.
    ///
    /// A block's subject isn't given its shape until the block closes, but it holds its place in
    /// template order from the moment it is opened — otherwise a `{{ ../outer }}` in the body would
    /// sort ahead of the list it sits inside.
    fn reserve(&mut self, path: &[&str]) -> Result<()> {
        let (head, rest) = match path.split_first() {
            Some(split) => split,
            None => return Err(ParseError::general("expected a variable name")),
        };
        if !rest.is_empty() {
            return self.object_at(head)?.reserve(rest);
        }
        self.index_of(head);
        Ok(())
    }

    /// Applies `mark` to the field at `path`, creating it and its parents as needed.
    fn touch(&mut self, path: &[&str], mark: Mark) -> Result<()> {
        let (head, rest) = match path.split_first() {
            Some(split) => split,
            None => return Err(ParseError::general("expected a variable name")),
        };
        if !rest.is_empty() {
            return self.object_at(head)?.touch(rest, mark);
        }
        let index = self.index_of(head);
        match mark {
            Mark::Value => self.fields[index].used_as_value = true,
            Mark::Condition => self.fields[index].used_as_condition = true,
        }
        Ok(())
    }

    /// Attaches a child scope produced by `{{#each}}` or `{{#with}}` to the field at `path`.
    fn attach(&mut self, path: &[&str], kind: FieldKind) -> Result<()> {
        let (head, rest) = match path.split_first() {
            Some(split) => split,
            None => return Err(ParseError::general("expected a variable name")),
        };
        if !rest.is_empty() {
            return self.object_at(head)?.attach(rest, kind);
        }
        let index = self.index_of(head);
        let existing = std::mem::replace(&mut self.fields[index].kind, FieldKind::Leaf);
        self.fields[index].kind = merge(head, existing, kind)?;
        Ok(())
    }

    /// Unions `other` into `self`, so two `{{#each rows}}` blocks over the same variable agree on
    /// one item type.
    fn absorb(&mut self, other: Context) -> Result<()> {
        self.used_as_value |= other.used_as_value;
        for field in other.fields {
            let index = self.index_of(&field.name);
            let mine = &mut self.fields[index];
            mine.used_as_value |= field.used_as_value;
            mine.used_as_condition |= field.used_as_condition;
            let existing = std::mem::replace(&mut mine.kind, FieldKind::Leaf);
            mine.kind = merge(&field.name, existing, field.kind)?;
        }
        Ok(())
    }
}

/// Reconciles two things the template said about the same variable.
fn merge(name: &str, existing: FieldKind, incoming: FieldKind) -> Result<FieldKind> {
    Ok(match (existing, incoming) {
        (FieldKind::Leaf, other) | (other, FieldKind::Leaf) => other,
        (FieldKind::Sequence(mut a), FieldKind::Sequence(b)) => {
            a.absorb(b)?;
            FieldKind::Sequence(a)
        }
        (FieldKind::Object(mut a), FieldKind::Object(b)) => {
            a.absorb(b)?;
            FieldKind::Object(a)
        }
        (FieldKind::Sequence(_), FieldKind::Object(_))
        | (FieldKind::Object(_), FieldKind::Sequence(_)) => {
            return Err(ParseError::general(&format!(
                "`{}` is used both as a list and as a record — it cannot be both",
                name
            )));
        }
    })
}

#[derive(Clone, Copy)]
enum Mark {
    Value,
    Condition,
}

/// Where a template reference points once `../` and any block alias have been applied.
enum Target {
    /// The scope itself — `{{ this }}`, or the alias of an enclosing `{{#each … as |x|}}`.
    Scope(usize),
    /// A field at `path` within the scope at `frame`.
    Field { frame: usize, path: Vec<String> },
}

/// One lexical scope: the root, or the body of an `{{#each}}` / `{{#with}}`.
struct Frame {
    /// The name bound by `as |x|`, if the block used one.
    alias: Option<String>,
    /// Where in the enclosing scope this block's subject lives.
    subject: Vec<String>,
    /// Which enclosing frame `subject` is relative to.
    parent: usize,
    /// `{{#each}}` produces a sequence; `{{#with}}` an object.
    sequence: bool,
    context: Context,
}

/// An open block, remembered so `{{/…}}` knows what to unwind and errors can point back at it.
struct Opened<'a> {
    /// The helper name — `each`, `if`, … — as the template spelled it.
    name: &'a str,
    /// The opening expression, so an error can report where the block began.
    raw: &'a str,
    /// Whether this block introduced a scope, or is transparent like `{{#if}}`.
    scope: bool,
}

/// Reads a template and returns the shape of the data it needs.
pub fn build(src: &str) -> Result<Context> {
    let mut frames = vec![Frame {
        alias: None,
        subject: Vec::new(),
        parent: 0,
        sequence: false,
        context: Context::default(),
    }];
    let mut opened: Vec<Opened<'_>> = Vec::new();

    let mut expression = Expression::from(src)?;
    while let Some(expr) = expression {
        match expr.expression_type {
            ExpressionType::Raw | ExpressionType::HtmlEscaped => {
                let content = expr.content.trim();
                if content != "else" {
                    reject_unsupported(content, &expr)?;
                    scan_expression(&mut frames, expr.content, Mark::Value, &expr)?;
                }
            }
            ExpressionType::Open => open_block(&mut frames, &mut opened, &expr)?,
            ExpressionType::Close => close_block(&mut frames, &mut opened, &expr)?,
            _ => {}
        }
        expression = expr.next()?;
    }

    if let Some(block) = opened.last() {
        return Err(ParseError::general(&format!(
            "`{{{{#{}}}}}` is never closed — it needs a matching `{{{{/{}}}}}`",
            block.name, block.name
        ))
        .or_at(block.raw));
    }
    Ok(frames.pop().expect("root frame is never popped").context)
}

/// Catches Handlebars this crate does not support yet, before it reaches code generation.
///
/// Without this these become Rust syntax errors in generated code, which say nothing to the person
/// who wrote the template. Partials are resolved before this point, by
/// [`crate::assemble`].
fn reject_unsupported(content: &str, expr: &Expression<'_>) -> Result<()> {
    if content == "else if" || content.starts_with("else if ") {
        return Err(ParseError::new(
            "`{{else if}}` is not supported yet — nest an `{{#if}}` inside the `{{else}}` instead",
            expr,
        ));
    }
    Ok(())
}

/// Records every variable an expression reads.
///
/// A bare `{{ name }}` is a variable. Anything with a head plus arguments is a helper call, which
/// this crate does not have — see [`check_helper`].
fn scan_expression(
    frames: &mut [Frame],
    content: &str,
    mark: Mark,
    expr: &Expression<'_>,
) -> Result<()> {
    let token = match Token::first(content)? {
        Some(token) => token,
        None => return Ok(()),
    };
    if let TokenType::SubExpression(_) = token.token_type {
        return Err(unsupported("a sub-expression like `(helper arg)`", expr));
    }
    match token.next()? {
        // Head plus arguments: a helper call. Only the arguments name data.
        Some(first_arg) => {
            check_helper(&token, expr)?;
            let mut arg = first_arg;
            loop {
                scan_token(frames, &arg, mark, expr)?;
                arg = match arg.next()? {
                    Some(next) => next,
                    None => return Ok(()),
                };
            }
        }
        None => scan_token(frames, &token, mark, expr),
    }
}

fn scan_token(
    frames: &mut [Frame],
    token: &Token<'_>,
    mark: Mark,
    expr: &Expression<'_>,
) -> Result<()> {
    match token.token_type {
        TokenType::Variable => record(frames, token.value, mark),
        TokenType::SubExpression(_) => {
            Err(unsupported("a sub-expression like `(helper arg)`", expr))
        }
        // A block supplies these rather than the caller, so none of them names data.
        TokenType::PrivateVariable => check_private(token.value, expr),
        TokenType::Literal => Ok(()),
    }
}

/// The `@…` variables a block can supply.
const PRIVATE: [&str; 1] = ["index"];

/// Rejects a helper call, rather than emitting a call to a function that does not exist and letting
/// the Rust compiler complain about it.
///
/// There are no inline helpers. A helper is Rust code, and a template that needs Rust code stops
/// being something a designer can own — so anything a helper would do belongs in the wiring instead.
fn check_helper(head: &Token<'_>, expr: &Expression<'_>) -> Result<()> {
    if let TokenType::Variable = head.token_type {
        return Err(ParseError::new(
            &format!(
                "`{}` is a helper, and helpers are out of scope — they would be Rust code inside a \
                 template. Do it in Rust when you set the value instead, e.g. \
                 `format!(\"{{:.2}}\", price)`",
                head.value
            ),
            expr,
        ));
    }
    Ok(())
}

/// Rejects an `@…` variable that no block supplies.
fn check_private(name: &str, expr: &Expression<'_>) -> Result<()> {
    let name = name.trim_start_matches("../");
    if PRIVATE.contains(&name) {
        return Ok(());
    }
    Err(unsupported(&format!("`@{}`", name), expr))
}

/// A construct that is real Handlebars but not implemented here.
fn unsupported(what: &str, expr: &Expression<'_>) -> ParseError {
    ParseError::new(&format!("{} is not supported yet", what), expr)
}

/// Notes that the template reads `var`, in whichever scope `var` resolves to.
fn record(frames: &mut [Frame], var: &str, mark: Mark) -> Result<()> {
    match resolve(frames, var)? {
        Target::Scope(frame) => {
            if let Mark::Value = mark {
                frames[frame].context.used_as_value = true;
            }
            Ok(())
        }
        Target::Field { frame, path } => {
            let path: Vec<&str> = path.iter().map(String::as_str).collect();
            frames[frame].context.touch(&path, mark)
        }
    }
}

/// Applies `../` and block aliases to work out what a reference actually points at.
fn resolve(frames: &[Frame], var: &str) -> Result<Target> {
    let mut index = frames.len() - 1;
    let mut var = var;

    while let Some(rest) = var.strip_prefix("../") {
        if index == 0 {
            return Err(ParseError::general(&format!(
                "`{}` reaches above the top of the template",
                var
            )));
        }
        index -= 1;
        var = rest;
    }

    loop {
        if index == 0 {
            break;
        }
        match &frames[index].alias {
            Some(alias) => {
                if var == alias {
                    return Ok(Target::Scope(index));
                }
                match var.strip_prefix(alias.as_str()) {
                    // `{{ row.name }}` inside `{{#each rows as |row|}}`.
                    Some(rest) if rest.starts_with('.') => {
                        var = &rest[1..];
                        break;
                    }
                    // A named block does not capture bare names: they mean the enclosing scope.
                    _ => index -= 1,
                }
            }
            None => {
                if var == "this" {
                    return Ok(Target::Scope(index));
                }
                if let Some(rest) = var.strip_prefix("this.") {
                    var = rest;
                }
                break;
            }
        }
    }

    if var.is_empty() {
        return Err(ParseError::general("expected a variable name"));
    }
    Ok(Target::Field {
        frame: index,
        path: var.split('.').map(str::to_string).collect(),
    })
}

fn open_block<'a>(
    frames: &mut Vec<Frame>,
    opened: &mut Vec<Opened<'a>>,
    expr: &Expression<'a>,
) -> Result<()> {
    let head = Token::first(expr.content)?
        .ok_or_else(|| ParseError::new("expected a block helper name", expr))?;

    let sequence = match head.value {
        "each" => true,
        "with" => false,
        "if" | "unless" => {
            if let Some(subject) = head.next()? {
                scan_token(frames, &subject, Mark::Condition, expr)?;
            }
            opened.push(Opened {
                name: head.value,
                raw: expr.raw,
                scope: false,
            });
            return Ok(());
        }
        other => {
            return Err(ParseError::new(
                &format!("unsupported block helper `{}`", other),
                expr,
            ));
        }
    };

    let subject = head.next()?.ok_or_else(|| {
        ParseError::new(
            &format!("`{{{{#{}}}}}` needs something to work on", head.value),
            expr,
        )
    })?;

    let alias = read_alias(&subject, expr)?;

    let (parent, path) = match resolve(frames, subject.value)? {
        Target::Field { frame, path } => (frame, path),
        Target::Scope(_) => {
            return Err(ParseError::new(
                &format!("`{{{{#{} this}}}}` is not supported yet", head.value),
                expr,
            ));
        }
    };

    {
        let borrowed: Vec<&str> = path.iter().map(String::as_str).collect();
        frames[parent].context.reserve(&borrowed)?;
    }

    frames.push(Frame {
        alias,
        subject: path,
        parent,
        sequence,
        context: Context::default(),
    });
    opened.push(Opened {
        name: head.value,
        raw: expr.raw,
        scope: true,
    });
    Ok(())
}

fn close_block(
    frames: &mut Vec<Frame>,
    opened: &mut Vec<Opened<'_>>,
    expr: &Expression<'_>,
) -> Result<()> {
    let block = match opened.pop() {
        Some(block) => block,
        None => {
            return Err(ParseError::new(
                &format!(
                    "`{{{{/{}}}}}` closes a block that was never opened",
                    expr.content.trim()
                ),
                expr,
            ));
        }
    };

    let closing = expr.content.trim();
    if closing != block.name {
        return Err(ParseError::new(
            &format!(
                "`{{{{/{}}}}}` does not match the `{{{{#{}}}}}` it closes",
                closing, block.name
            ),
            expr,
        ));
    }

    if !block.scope {
        return Ok(());
    }

    let frame = frames.pop().expect("a scope entry always has a frame");
    let path: Vec<&str> = frame.subject.iter().map(String::as_str).collect();
    let kind = if frame.sequence {
        FieldKind::Sequence(frame.context)
    } else {
        FieldKind::Object(frame.context)
    };
    frames[frame.parent].context.attach(&path, kind)
}

/// Reads the `as |name|` clause of a block, if it has one.
fn read_alias(subject: &Token<'_>, expr: &Expression<'_>) -> Result<Option<String>> {
    let keyword = match subject.next()? {
        Some(token) => token,
        None => return Ok(None),
    };
    if keyword.value != "as" {
        return Err(ParseError::new(
            &format!("unexpected `{}`", keyword.value),
            expr,
        ));
    }
    let mut token = keyword;
    loop {
        token = token
            .next()?
            .ok_or_else(|| ParseError::new("expected a name after `as`", expr))?;
        if token.value == "|" {
            continue;
        }
        return Ok(Some(token.value.trim_matches('|').to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(src: &str) -> Context {
        build(src).unwrap_or_else(|e| panic!("failed to build context for {:?}: {}", src, e))
    }

    fn names(context: &Context) -> Vec<&str> {
        context.fields.iter().map(|f| f.name.as_str()).collect()
    }

    fn field<'a>(context: &'a Context, name: &str) -> &'a Field {
        context
            .fields
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("no field named {}", name))
    }

    fn sequence<'a>(context: &'a Context, name: &str) -> &'a Context {
        match &field(context, name).kind {
            FieldKind::Sequence(inner) => inner,
            other => panic!("expected {} to be a sequence, got {:?}", name, other),
        }
    }

    fn object<'a>(context: &'a Context, name: &str) -> &'a Context {
        match &field(context, name).kind {
            FieldKind::Object(inner) => inner,
            other => panic!("expected {} to be an object, got {:?}", name, other),
        }
    }

    #[test]
    fn top_level_variables_in_template_order() {
        let context = ctx("<p>{{firstname}} {{lastname}}</p>");
        assert_eq!(names(&context), ["firstname", "lastname"]);
        assert!(context.fields.iter().all(|f| f.used_as_value));
        assert!(context.fields.iter().all(|f| f.kind == FieldKind::Leaf));
    }

    #[test]
    fn a_variable_used_twice_is_recorded_once() {
        let context = ctx("{{name}} {{name}} {{{name}}}");
        assert_eq!(names(&context), ["name"]);
    }

    #[test]
    fn dotted_paths_become_nested_objects() {
        let context = ctx("{{person.firstname}} {{person.lastname}}");
        assert_eq!(names(&context), ["person"]);
        assert_eq!(names(object(&context, "person")), ["firstname", "lastname"]);
    }

    #[test]
    fn deeply_dotted_paths_nest_all_the_way_down() {
        let context = ctx("{{a.b.c}}");
        let c = object(object(&context, "a"), "b");
        assert_eq!(names(c), ["c"]);
        assert!(field(c, "c").used_as_value);
    }

    #[test]
    fn each_records_the_item_shape() {
        let context = ctx("{{#each rows}}<li>{{name}}</li>{{/each}}");
        assert_eq!(names(&context), ["rows"]);
        let item = sequence(&context, "rows");
        assert_eq!(names(item), ["name"]);
        assert!(!item.used_as_value);
    }

    #[test]
    fn each_over_scalars_marks_the_item_itself() {
        let context = ctx("{{#each tags}}<span>{{this}}</span>{{/each}}");
        let item = sequence(&context, "tags");
        assert!(item.used_as_value);
        assert!(item.is_scalar());
    }

    #[test]
    fn nested_each_nests_the_item_contexts() {
        let context = ctx("{{#each rows}}{{#each cells}}{{value}}{{/each}}{{/each}}");
        let row = sequence(&context, "rows");
        assert_eq!(names(row), ["cells"]);
        assert_eq!(names(sequence(row, "cells")), ["value"]);
    }

    #[test]
    fn an_alias_names_the_item_and_frees_bare_names() {
        let context = ctx("{{#each rows as |row|}}{{row.name}} {{title}}{{/each}}");
        // `title` is not captured by the alias, so it belongs to the enclosing scope.
        assert_eq!(names(&context), ["rows", "title"]);
        assert_eq!(names(sequence(&context, "rows")), ["name"]);
    }

    #[test]
    fn an_alias_used_bare_marks_the_item_as_a_value() {
        let context = ctx("{{#each tags as |tag|}}{{tag}}{{/each}}");
        assert!(sequence(&context, "tags").used_as_value);
    }

    #[test]
    fn parent_references_walk_outwards() {
        let context = ctx("{{#each rows}}{{name}} of {{../company}}{{/each}}");
        assert_eq!(names(&context), ["rows", "company"]);
        assert_eq!(names(sequence(&context, "rows")), ["name"]);
    }

    #[test]
    fn parent_references_walk_out_more_than_one_level() {
        let context = ctx("{{#each a}}{{#each b}}{{../../top}}{{/each}}{{/each}}");
        assert_eq!(names(&context), ["a", "top"]);
    }

    #[test]
    fn with_records_an_object() {
        let context = ctx("{{#with author}}{{first_name}} {{last_name}}{{/with}}");
        assert_eq!(names(&context), ["author"]);
        assert_eq!(
            names(object(&context, "author")),
            ["first_name", "last_name"]
        );
    }

    #[test]
    fn if_marks_a_condition_without_making_it_a_value() {
        let context = ctx("{{#if has_author}}<h1>x</h1>{{/if}}");
        let field = field(&context, "has_author");
        assert!(field.used_as_condition);
        assert!(!field.used_as_value);
    }

    #[test]
    fn a_variable_can_be_both_tested_and_displayed() {
        let context = ctx("{{#if title}}<h1>{{title}}</h1>{{/if}}");
        let field = field(&context, "title");
        assert!(field.used_as_condition);
        assert!(field.used_as_value);
        assert_eq!(field.kind, FieldKind::Leaf);
    }

    #[test]
    fn if_does_not_open_a_scope() {
        let context = ctx("{{#if shown}}{{name}}{{/if}}");
        assert_eq!(names(&context), ["shown", "name"]);
    }

    #[test]
    fn unless_marks_a_condition() {
        let context = ctx("{{#unless hidden}}x{{/unless}}");
        assert!(field(&context, "hidden").used_as_condition);
    }

    #[test]
    fn a_list_can_be_tested_and_iterated() {
        let context = ctx("{{#if rows}}{{#each rows}}{{name}}{{/each}}{{/if}}");
        let field = field(&context, "rows");
        assert!(field.used_as_condition);
        assert_eq!(names(sequence(&context, "rows")), ["name"]);
    }

    #[test]
    fn repeated_each_blocks_merge_into_one_item_shape() {
        let context = ctx("{{#each rows}}{{a}}{{/each}}{{#each rows}}{{b}}{{/each}}");
        assert_eq!(names(sequence(&context, "rows")), ["a", "b"]);
    }

    #[test]
    fn each_over_a_dotted_path() {
        let context = ctx("{{#each report.rows}}{{name}}{{/each}}");
        assert_eq!(names(&context), ["report"]);
        let rows = object(&context, "report");
        assert_eq!(names(sequence(rows, "rows")), ["name"]);
    }

    /// A helper is Rust code, so a template cannot call one. Rejecting it here means the developer
    /// gets a message about their template rather than "cannot find function" from rustc.
    #[test]
    fn a_helper_call_is_rejected_by_name() {
        let err = build(r#"Price: ${{format "{:.2}" price}}"#).unwrap_err();
        assert!(
            err.to_string().contains("format") && err.to_string().contains("helper"),
            "error should name the helper: {}",
            err
        );
    }

    #[test]
    fn else_is_not_a_variable() {
        let context = ctx("{{#if a}}x{{else}}y{{/if}}");
        assert_eq!(names(&context), ["a"]);
    }

    #[test]
    fn comments_contribute_nothing() {
        let context = ctx("{{! ignored }}{{name}}{{!-- {{also_ignored}} --}}");
        assert_eq!(names(&context), ["name"]);
    }

    #[test]
    fn private_variables_are_supplied_by_the_block() {
        let context = ctx("{{#each rows}}{{@index}}{{name}}{{/each}}");
        assert_eq!(names(sequence(&context, "rows")), ["name"]);
    }

    #[test]
    fn a_list_used_as_a_record_is_an_error() {
        let err = build("{{#each rows}}{{a}}{{/each}}{{rows.name}}").unwrap_err();
        assert!(
            err.to_string().contains("rows"),
            "error should name the variable: {}",
            err
        );
    }

    #[test]
    fn an_unknown_block_helper_is_named_in_the_error() {
        let err = build("{{#wat x}}y{{/wat}}").unwrap_err();
        assert!(
            err.to_string().contains("wat"),
            "error should name the helper: {}",
            err
        );
    }

    #[test]
    fn a_stray_closing_tag_is_an_error() {
        assert!(build("{{/if}}").is_err());
    }

    #[test]
    fn an_unclosed_block_is_an_error() {
        assert!(build("{{#each rows}}{{name}}").is_err());
    }
}
