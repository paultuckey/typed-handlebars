# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Releases are cut with `cargo release`, which turns the `[Unreleased]` heading below into the
version being released. See [docs/Release.md](docs/Release.md).

## [Unreleased]

### Added

- **Helpers: `{{ t "Save" }}` calls a method on a frame you name.** Handlebars passes a template two
  things — the data, and a *data frame* of ambient state supplied at render time (`options.data` in
  handlebars.js, which is where a translation helper reads its locale from). `Vars` was already the
  data; `register_helper!` now names the frame, and a helper is one of its methods:

  ```rust
  impl Ctx {
      pub fn t(&self, key: &str) -> String { self.locale.lookup(key) }
  }

  mod templates {
      typed_handlebars::register_helper!(crate::Ctx);
      typed_handlebars::directory!("templates/");
  }

  templates::page::Vars { .. }.render(&ctx)
  ```

  This replaces the previous position, which was that a helper "would be Rust code inside a
  template". `{{ t "Save" }}` is not Rust — the argument is a string literal — and a designer who
  wants a translated button had otherwise to ask a Rust developer to invent a `t_save` variable and
  wire it up. Method resolution is what checks the name, so a helper that does not exist is
  `no method named 't' found for struct 'Ctx'` rather than something that renders wrongly.

  Only a template that calls a helper takes the frame, so adding one to a template — or to a partial
  it includes — is what makes its call sites ask for it, exactly as adding a variable does.

  Every argument arrives as a `&str`: a quoted string or a number is the text the template spelled,
  so `{{ money 123 }}` calls `money("123")`, and anything else is a variable written out the way
  `{{{ … }}}` would write it. Arity is the method's business. The result is escaped by `{{ }}` and
  not by `{{{ }}}`, matching handlebars.js unless a JS shim returns a `SafeString`.

- **Single-quoted string literals.** `{{ t 'Save' }}` is what handlebars.js accepts alongside
  `{{ t "Save" }}`, and a designer has no reason to know which was easier to lex.

### Changed

- **A hash argument is now rejected by name.** `{{ t "Hello" name=user }}` is how handlebars.js
  passes named arguments and is not supported yet; it previously lexed as a variable *called*
  `name=user`, generating a field that compiled, asked the caller for something meaningless and
  rendered the wrong thing.

- **`lookup` and `log` are reserved**, alongside the block names, so the same spelling cannot mean
  a builtin in one template and a frame method in another. `{{lookup a b}}` was already unsupported;
  it now says why rather than reaching code generation.

## [0.3.0] — 2026-08-16

### Changed

- **A template is wired up by writing its variables as a struct literal.** Each template's module
  now holds a `Vars` — every variable it uses, named — which is a plain struct you write directly:

  ```rust
  templates::button::Vars { btn_id: 42, btn_name: "Save" }.render()
  ```

  This replaces the positional function, which was the one wiring path where a mistake could not be
  caught: arguments went in "the order the template first mentions them", so reordering the markup
  reordered the arguments, and two variables of the same type swapped with no compile error. That is
  an edit a template author makes without knowing Rust is downstream. `Vars` is checked by the
  compiler's own diagnostics — `E0560` names a misspelled field and suggests the right one, `E0063`
  names a forgotten one, `E0062` catches a repeat — and, being exhaustive, a variable *added* to a
  `.hbs` now breaks every call site instead of quietly rendering as nothing.

  It is also much closer to Handlebars itself, where a template is called with one object: the
  context. Nothing in Handlebars has ever had a concept of variables in source order.

- **The builder is reached through `builder()`.** `templates::button::Builder::new()` becomes
  `templates::button::builder()`, and a nested type's builder moves from `RowsItemBuilder::new()` to
  `RowsItem::builder()`. The builder is unchanged otherwise, and is now the answer to "I do not have
  every variable" — a struct literal cannot leave a field out, so this is the form that expresses
  what an undefined variable does in Handlebars.

- **Generated types carry no marker parameters.** `Template<i64, ViaDisplay, bool, &String, ViaDisplay>`
  is now `Vars<i64, bool, &String>`: one parameter per field and nothing else. The `Render` markers
  that say *how* a value is written moved onto `render`/`render_to` as method-level generics, which
  also removes the `PhantomData` that held them — and that is what lets the struct be written as a
  literal at all. An `Option` no longer needs its marker spelled, so the rule that every marker
  before it had to be spelled too is gone with it.

  A list's item parameters likewise stop threading up into the parent, since an item type is named
  only in an `AsRef` bound: `deep::RowsItem<&str, Vec<deep::RowsItemCellsItem<i32>>>` has two
  parameters, not three.

### Fixed

- **A variable may be called `builder`, `vars`, or anything else the generator also names.**
  `{{ builder.name }}` is ordinary Handlebars, but it camel-cases onto a type the module generates,
  and the result was `E0428: the name Builder is defined multiple times` — a wall of Rust errors
  against a template that is not wrong about anything, which is precisely the error a template
  author cannot read. Generated type names are now handed out in template order, and one that finds
  its name taken takes a trailing underscore instead, the same escape a Rust keyword gets. The
  module's own `Vars` and `Builder` are reserved ahead of everything, so they always mean what a
  caller expects.

  This also covers two variables colliding with each other — `{{ rows_item.x }}` beside
  `{{#each rows}}`, or `{{ person_builder.x }}` beside `{{ person.y }}` — since a type and the
  builder it brings with it are reserved together.

  A template file called `builder.hbs` or `vars.hbs` needed no escape and gets none: the `builder()`
  function lives inside the template's own module, so `templates::builder::builder()` is not a
  clash.

### Removed

- **`Display` for template values, and with it zero-copy nesting.** A template value could be passed
  straight into a parent's `{{{ }}}` and written into the same buffer; markers must be type
  parameters for `Display::fmt` to be implementable, and they had to leave the type for the struct
  literal to work. Nest by passing the rendered `String` instead:

  ```rust
  templates::page::Vars { content: templates::row::Vars { name: "King" }.render() }
  ```

  This is how handlebars.js composes too — render the fragment, pass the markup in as a variable —
  and it costs one allocation per nesting site. The paths where that would multiply, `{{#each}}` and
  `{{> partial}}`, are spliced at compile time and allocate nothing either way.

- **`Template::new` and `RowsItem::new`.** Both were positional; write the struct literal.

## [0.2.0] — 2026-08-15

### Added

- **`Option` renders, and `None` renders as nothing** — as null and undefined do in handlebars.js.
  `{{ x }}` on a nullable value used to be a compile error, which left `{{#if x}}` working on an
  `Option` that could not then be printed, and left callers writing
  `x.as_deref().unwrap_or("")` at the call site. It now works by value or by reference, in a
  record, as a `{{#each}}` item, and through a builder setter. `false` and `0` remain values and
  still render as `false` and `0`.

- **`{{@root.…}}` reaches the template's top-level context** from any depth — inside `{{#each}}`,
  inside `{{#with}}`, nested in both, and as a block subject (`{{#each @root.rows}}`). Unlike
  `{{@index}}`, `{{@first}}` and `{{@last}}`, which are loop state, `@root` is absolute:
  `{{@../root.title}}` reads the same value, as it does in handlebars.js, so the prefix is stripped
  rather than walked. A partial sees the including template's root. Bare `{{@root}}` is a compile
  error naming the construct — handlebars.js writes `[object Object]` for the whole context, and
  there is nothing useful to write instead. Iterating any other `@…` (`{{#each @index}}`) is now a
  named error too, rather than quietly becoming a record called `index`.
- **`{{ rows.length }}` counts a list**, as it does in handlebars.js, and a variable can be counted
  and iterated at once. It used to become a record named `Rows` with a `length` field — a type that
  compiled, asked the caller for something meaningless and rendered the wrong thing — or, when the
  template also had `{{#each rows}}`, an error claiming `rows` could not be both a list and a
  record, which in handlebars.js it can. `{{#if rows.length}}` tests the count. Only lists have a
  `.length`: a `String` is a compile error naming the value, because JS counts UTF-16 code units
  where Rust counts bytes or `char`s. A record field genuinely named `length` is now unreachable —
  handlebars.js resolves that by runtime type, and a compile-time generator has to pick one.
- **A list left unset counts as nothing rather than `0`.** Absent and empty are the same everywhere
  else, but handlebars.js writes nothing for `undefined.length` and `0` for `[].length`, so the
  unset placeholder for a list is now `Absent<Item>` rather than `[Item; 0]`.

### Changed

- **`[…]` path segments are a compile error naming the construct**, rather than silently becoming a
  record field. This covers indexing (`{{ rows.[0] }}`) and quoted names (`{{ [odd name] }}`);
  neither is implemented, and both used to generate a nonsense type.
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

[Unreleased]: https://github.com/paultuckey/typed-handlebars/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/paultuckey/typed-handlebars/releases/tag/v0.3.0
[0.2.0]: https://github.com/paultuckey/typed-handlebars/releases/tag/v0.2.0
[0.1.0]: https://github.com/paultuckey/typed-handlebars/releases/tag/v0.1.0
