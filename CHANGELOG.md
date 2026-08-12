# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — unreleased

Initial release. The crate began as a fork of the parser from
[rusty-handlebars](https://github.com/h-i-v-e/rusty-handlebars); the notes below describe how it
behaves now, and call out the places where that behaviour differs from its origin.

### Added

- **Types inferred from the template.** `{{#each rows}}{{ name }}{{/each}}` says `rows` is a list
  of records with a `name`, so the macro generates that type. Nothing is declared in Rust and
  nothing is declared in the `.hbs` file; `{{#each}}`, `{{#with}}` and `{{ person.name }}` work
  from `directory!` and `file!` rather than only from `str!`.
- **A builder per template**, alongside the positional constructor. Setters are named, so argument
  order stops mattering and renaming a template variable becomes a compile error. Unset means
  empty, as an undefined variable does in Handlebars.
- **Partials** (`{{> row}}`), resolved at compile time by splicing, with handlebars.js's inherited
  context. No second `render` call and no intermediate `String`. Editing a partial rebuilds every
  template that includes it.
- **`Display` and `render_to`.** A nested template writes straight into its parent's buffer, so
  nesting no longer allocates a `String` per level.
- **Template diagnostics in Handlebars terms**, with the `.hbs` path, line and column — including
  for mistakes inside a spliced partial. Under `directory!`, one broken template no longer stops
  the others compiling.
- **A documented supported subset**, with every unsupported construct failing by name rather than
  as a Rust type error. `reference-ts` checks the supported constructs against real handlebars.js.
- **A module per template**, mirroring the directory layout, so `templates/admin/row.hbs` and
  `templates/public/row.hbs` no longer collide.
- Template names that are Rust keywords (`{{ type }}`, `mod.hbs`) or start with a digit
  (`{{ 2nd }}`, `2col.hbs`) are mangled into usable identifiers instead of failing.

### Changed

- **`{{ }}` now HTML-escapes and `{{{ }}}` does not**, as Handlebars specifies. Previously both
  emitted identical code, so `{{ }}` passed markup through unescaped. **If you relied on the old
  behaviour, move that value to `{{{ }}}`.** Escaping covers the same characters handlebars.js
  escapes — `&`, `<`, `>`, `"`, `'`, `` ` `` and `=` — and happens as the value is written.
- **`{{#if}}` and `{{#unless}}` use Handlebars truthiness** rather than forcing the tested variable
  to `bool`, so `{{#if title}}{{title}}{{/if}}` compiles and behaves. Absent, `false`, `""`, `0`
  and an empty list are falsy.
- Generated code is fully self-contained: every emitted path is absolute, no `use` statements reach
  the caller's scope, and the expansion is lint-clean in a crate denying `missing_docs`,
  `unused` and `clippy::pedantic`.
- Generated code resolves the runtime crate through `proc-macro-crate`, so renaming the dependency
  works.
- `WalkDir` is sorted, so generated code no longer varies with filesystem order.

### Removed

- **The Rust-side type override** — `str!("x", "…", ("rows", Vec<Row>))`. Every type now comes from
  the template. It only ever worked in `str!`, so it was a Rust-only affordance missing from the
  headline entry point.
- **`{{format "…" x}}`**, which was never a Handlebars helper and whose first argument was a Rust
  format spec — the one place Rust syntax leaked into a `.hbs` file. Formatting is wiring:
  `.price(format!("{:.2}", price))`.

### Known divergences from handlebars.js

- `{{#with person}}` renders its block even when `person` was never set, showing empty fields,
  where handlebars.js skips the block. `{{#if}}` and `{{#unless}}` are unaffected.
- `{{{{raw}}}}…{{{{/raw}}}}` always passes its content through; handlebars.js treats the name as a
  helper and renders nothing when none is registered.
- Standalone partials are not re-indented.

[Unreleased]: https://github.com/paultuckey/typed-handlebars/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/paultuckey/typed-handlebars/releases/tag/v0.1.0
