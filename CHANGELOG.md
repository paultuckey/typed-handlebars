# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Releases are cut with `cargo release`, which turns the `[Unreleased]` heading below into the
version being released. See [docs/Release.md](docs/Release.md).

## [Unreleased]

### Added

- **`Option` renders, and `None` renders as nothing** — as null and undefined do in handlebars.js.
  `{{ x }}` on a nullable value used to be a compile error, which left `{{#if x}}` working on an
  `Option` that could not then be printed, and left callers writing
  `x.as_deref().unwrap_or("")` at the call site. It now works by value or by reference, in a
  record, as a `{{#each}}` item, and through a builder setter. `false` and `0` remain values and
  still render as `false` and `0`.

### Changed

- **Written values are bound by `Render` rather than `Display`.** A leaf the template writes out
  carries a second, inference-filled marker parameter naming which `Render` impl its value takes,
  held in the type's `PhantomData`. This is what allows an `Option` to render at all: Rust's
  coherence rules forbid a crate from writing both `impl<T: Display> Render for T` and
  `impl<T> Render for Option<T>`. Call sites are unaffected — inference fills the marker in — but
  generated type signatures carry the extra parameter, and `escape` is replaced by
  `RenderExt::escaped` and `RenderExt::shown`.

## [0.1.0] — 2026-08-15

First release. `typed-handlebars` turns `.hbs` files into Rust at compile time: the types a
template needs are generated from what the template itself says, so nothing is declared twice and
there is no parsing, registry or lookup at run time.

### Added

- **Types inferred from the template.** `{{#each rows}}{{ name }}{{/each}}` says `rows` is a list
  of records with a `name`, so the macro generates that type. Nothing is declared in Rust and
  nothing is declared in the `.hbs` file.
- **Three entry points** — `directory!`, `file!` and `str!` — with `directory!` mirroring a folder
  of templates into a module tree, so `templates/admin/row.hbs` and `templates/public/row.hbs` do
  not collide.
- **A builder per template**, alongside the positional constructor. Setters are named, so argument
  order stops mattering and renaming a template variable becomes a compile error. Unset means
  empty, as an undefined variable does in Handlebars.
- **Partials** (`{{> row}}`), resolved at compile time by splicing, with handlebars.js's inherited
  context. No second `render` call and no intermediate `String`. Editing a partial rebuilds every
  template that includes it.
- **`Display` and `render_to`.** A nested template writes straight into its parent's buffer, so
  nesting no longer allocates a `String` per level.
- **Handlebars-accurate escaping.** `{{ }}` HTML-escapes and `{{{ }}}` does not, covering the same
  characters handlebars.js escapes — `&`, `<`, `>`, `"`, `'`, `` ` `` and `=` — as the value is
  written.
- **Handlebars truthiness** in `{{#if}}` and `{{#unless}}`, so `{{#if title}}{{title}}{{/if}}`
  compiles and behaves. Absent, `false`, `""`, `0` and an empty list are falsy. This includes
  testing a loop item itself — `{{#each xs}}{{#if this}}…{{/if}}{{/each}}` — and the bound follows
  what the template asks for, so an item that is only tested need not be printable.
- **`{{@first}}` and `{{@last}}`** inside `{{#each}}`, alongside `{{@index}}`. They work as values
  and as conditions, so `{{#unless @last}}, {{/unless}}` between items does what it looks like.
  An `{{@…}}` can be read from anywhere inside the loop, including from within a nested `{{#if}}`
  or `{{#with}}`, and `../` steps out one loop rather than one block — both as in handlebars.js.
- **Standalone lines.** A line whose only content is a block tag, an `{{else}}` or a comment
  contributes nothing of its own: its indentation and its trailing newline both go, as in
  handlebars.js, so a template laid out over several lines renders as written instead of gaining a
  blank line after every tag. An interpolation is not standalone — `{{ name }}` alone on a line
  keeps its newline, because it is there to produce output. A partial alone on a line is standalone
  too, and its indentation is applied to every line it emits rather than dropped, so an included
  block of markup lines up where it was written.
- **Comments in all their forms**, including the trimming closes `{{! … ~}}` and `{{!-- … --~}}`,
  and empty comments (`{{!}}`, `{{!----}}`). A long comment ends at its first close, so a later
  `--~}}` is text.
- **`{{else if}}` and `{{else unless}}`**, chained onto `{{#if}}` and `{{#unless}}` to any depth.
  The chained helper sets its own sense, as in handlebars.js, so `{{else if}}` tests for truth even
  inside an `{{#unless}}`. Chaining onto `{{#each}}` or `{{#with}}`, or chaining a block that opens
  a scope, is a compile error naming the construct.
- **Template diagnostics in Handlebars terms**, with the `.hbs` path, line and column — including
  for mistakes inside a spliced partial. Under `directory!`, one broken template no longer stops
  the others compiling.
- **A documented supported subset**, with every unsupported construct failing by name rather than
  as a Rust type error. `reference-ts` checks the supported constructs against real handlebars.js.
- Generated code is self-contained: every emitted path is absolute, no `use` statements reach the
  caller's scope, the runtime crate is resolved through `proc-macro-crate` so renaming the
  dependency works, and the expansion is lint-clean in a crate denying `missing_docs`, `unused`
  and `clippy::pedantic`.
- Template names that are Rust keywords (`{{ type }}`, `mod.hbs`) or start with a digit
  (`{{ 2nd }}`, `2col.hbs`) are mangled into usable identifiers instead of failing.

### Known divergences from handlebars.js

- `{{#with person}}` renders its block even when `person` was never set, showing empty fields,
  where handlebars.js skips the block. `{{#if}}` and `{{#unless}}` are unaffected.
- `{{{{raw}}}}…{{{{/raw}}}}` always passes its content through; handlebars.js treats the name as a
  helper and renders nothing when none is registered.
- A **carriage return** in template text does not survive code generation, so a `.hbs` file saved
  with CRLF line endings renders with LF.

[Unreleased]: https://github.com/paultuckey/typed-handlebars/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/paultuckey/typed-handlebars/releases/tag/v0.1.0
