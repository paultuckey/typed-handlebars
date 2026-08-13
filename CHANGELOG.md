# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Releases are cut with `cargo release`, which turns the `[Unreleased]` heading below into the
version being released. See [docs/Release.md](docs/Release.md).

## [Unreleased]

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
  compiles and behaves. Absent, `false`, `""`, `0` and an empty list are falsy.
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
- Standalone partials are not re-indented.

[Unreleased]: https://github.com/paultuckey/typed-handlebars/compare/v0.1.0...HEAD
