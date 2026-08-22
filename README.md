# Typed Handlebars

[![crates.io](https://img.shields.io/crates/v/typed-handlebars.svg)](https://crates.io/crates/typed-handlebars)
[![docs.rs](https://docs.rs/typed-handlebars/badge.svg)](https://docs.rs/typed-handlebars)
[![CI](https://github.com/paultuckey/typed-handlebars/actions/workflows/ci.yml/badge.svg)](https://github.com/paultuckey/typed-handlebars/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/rustc-1.88+-blue.svg)](#minimum-supported-rust-version)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue.svg)](#licence)

_Experimental_ compile-time checked [Handlebars](https://handlebarsjs.com/) templates for [Rust](https://rust-lang.org/). Based on the parser
from [rusty-handlebars](https://github.com/h-i-v-e/rusty-handlebars).

[Code first or schema first](https://blog.logrocket.com/code-first-vs-schema-first-development-graphql/)
highlights that there are two way of thinking about templating. Code first or template first.

This library takes a _template first_ approach. The designer makes pure handlebars files (`hbs`) that can be edited
separately. Then the Rust developer gets a pure rust experience with compile time checking of templates and how they are
called from Rust.

The Rust developer should not have to repeat the template name or variable names in Rust code. They should be able to 
use autocomplete in their IDE.

## Getting started

```shell
cargo add typed-handlebars
```

Now make a directory of Handlebars files. eg, `templates/button.hbs`:

```handlebars
<button id="btn{{ btn_id }}" class="btn btn-light">
    {{ btn_name }}
</button>
```

Then in rust:

```rust
mod templates {
    typed_handlebars::directory!("templates/");
}
fn get_html() -> String {
    // templates::button module and struct is automatically generated
    templates::button::Vars { btn_id: 42, btn_name: "Save" }.render()
}
```

`Vars` is every variable the template uses, in an ordinary struct.

Your IDE can offer the names rather than you retyping them, and every mistake is caught where you made it:

```
error[E0560]: struct `button::Vars` has no field named `btn_nmae`
help: a field with a similar name exists: `btn_name`

error[E0063]: missing field `btn_name` in initializer of `button::Vars`
```

That last one is the point of naming them all: add `{{ subtitle }}` to the `.hbs` and every call
site stops compiling until it says what the subtitle is, rather than quietly rendering nothing.

### When you don't have every variable

Handlebars renders an undefined variable as nothing, and `builder()` is how you say that. Set what
you have; anything you leave out renders as empty, a list with no items, or a false condition:

```rust
templates::button::builder().btn_id(42).render()   // btn_name renders as nothing
```

So use: `Vars` when you have everything or `builder()` when you don't.

## Goals

**As much as possible at compile time, as little as possible at runtime.** Templates are turned into Rust when the crate
is built, so there is no parsing, no template registry and no lookups while your program runs — just the code the
template implies. Where a design choice trades build-time work against run-time work, build time wins.

**The Handlebars author needs to know no Rust.** An `.hbs` file is plain Handlebars, written by someone who never has to
think about what happens downstream. No Rust type names, no annotations, no macro-specific syntax — nothing in the
template that a designer could not write, or that would stop the same file rendering under handlebars.js given the
`registerHelper` calls it names.

**The code generator takes on the complexity.** The template already says what data it needs: `{{#each rows}}{{ name }}`
means a list of records with a `name`. The macro reads that and generates the types, so nobody has to declare them
twice. There are no traits for you to implement and nothing to derive.

**The Rust developer only does the wiring.** Connect your data to what the generator produced, with the names supplied
by IDE autocomplete rather than retyped from the template. Getting a name wrong should be a compile error, not something
you discover in a rendered page.


## What is supported

Still alpha. The table below is the whole of it — anything not listed is a compile error naming the
construct, never a silent difference and never a Rust type error you would have to decode.

### Works

| Construct                                   | Notes                                                                            |
|---------------------------------------------|----------------------------------------------------------------------------------|
| `{{ name }}`                                | HTML-escaped                                                                     |
| `{{{ name }}}`                              | raw, for markup you have already rendered                                        |
| `{{ person.name }}`                         | a `person` record is generated                                                   |
| `{{ rows.length }}`                         | how many items; countable and iterable at the same time                          |
| `{{ ../name }}`                             | reaches the enclosing scope                                                      |
| `{{@root.name}}`                            | reaches the top level from any depth; `{{@root}}` alone is an error              |
| `{{#if}}` / `{{#unless}}` / `{{else}}`      | Handlebars truthiness; testing a variable does not stop you printing it          |
| `{{else if}}` / `{{else unless}}`           | chained onto `{{#if}}` / `{{#unless}}`, to any depth                             |
| `{{#each rows}}`                            | with `{{this}}`, `{{@index}}` `{{@first}}` `{{@last}}`, `{{else}}`, `as \|row\|` |
| `{{#with person}}`                          | see the divergence below                                                         |
| `{{ t "Save" }}`                            | a helper (t): a method on the frame — see usage                                  |
| `{{> row}}`                                 | partials, rendered against the context they were included from                   |
| `{{! … }}` / `{{!-- … --}}`                 | comments, including the trimming closes `{{! … ~}}` and `{{!-- … --~}}`          |
| `{{~ … ~}}`                                 | whitespace trimming                                                              |
| a tag alone on a line                       | indentation and newline go; a partial's indent reaches its every line            |
| `\{{ … }}` and `{{{{raw}}}} … {{{{/raw}}}}` | literal output                                                                   |

### Not yet

`{{@key}}` `{{@value}}` · `{{lookup}}` ·
sub-expressions `( … )` · `{{#with}}` with `{{else}}` · partial arguments (`{{> row this}}`) ·
inline partials (`{{#*inline}}`) · `[…]` path segments, both indexing (`{{ rows.[0] }}`) and
quoted names (`{{ [odd name] }}`) · lists that are not slice-backed (`HashMap`, `VecDeque`) ·
hash arguments (`{{ t "Hello" name=user }}`) · block helpers (`{{#t}}Hello{{/t}}`) · a helper
anywhere but where its result is written, such as `{{#if (t "x")}}`.

### Out of scope

**Runtime template loading.** Templates are compiled into your binary, so there is nothing to load
and no dynamic partial names. The aim of this project is compile time safety.

### Partials

`{{> row}}` includes `row.hbs` from the same directory. As in handlebars.js, the partial renders
against the context it was included from, so this works with no extra wiring:

`templates/row.hbs`:

```handlebars
<li id="r{{ id }}">{{ name }}</li>
```

`templates/page.hbs`:

```handlebars
<ul>{{#each rows}}{{> row}}{{/each}}</ul>
```

The partial's variables become part of the including template, so `page` asks for rows of `id` and
`name` — you never name `row.hbs` in Rust. `row.hbs` still gets its own type, so it can be rendered
on its own too.

### Where the generated names live

Each template gets a module of its own, named after the file, holding the types it needs. The
directory layout becomes the module layout:

```
templates/page.hbs          templates::page::Vars              templates::page::RowsItem
templates/admin/row.hbs     templates::admin::row::Vars        templates::admin::row::builder()
```

So two templates called `row` in different directories are two different modules, rather than a
name collision.

Partials are resolved at compile time by splicing, so there is no second `render` call and no
intermediate `String`. Editing a partial rebuilds every template that includes it. Cycles, unknown
names and arguments (`{{> row this}}`, not supported yet) are all compile errors. Partials need a
directory to look in, so they work with `directory!` and `file!` but not `str!`.

### Escaping

`{{ name }}` HTML-escapes its value and `{{{ name }}}` does not, as Handlebars specifies. Escaping
covers the same characters handlebars.js escapes - `&`, `<`, `>`, `"`, `'`, `` ` `` and `=` - and
happens as the value is written, so nothing is allocated for it.


### Known divergence from handlebars.js

`{{#with person}}` renders its block even when `person` was never set, showing empty fields, where handlebars.js would
skip the block. `{{#if}}` and `{{#unless}}` are unaffected — an absent variable is correctly falsy there.


## Minimum supported Rust version

1.88, verified in CI by building both published crates on that toolchain rather than assumed from
the edition — edition 2024 needs only 1.85, but a let-chain in the parser and the `trybuild`
dev-dependency both need 1.88. Raising the MSRV is treated as a breaking change.

## See also

[Usage notes](https://github.com/paultuckey/typed-handlebars/blob/main/docs/Usage.md) ·
[Changelog](https://github.com/paultuckey/typed-handlebars/blob/main/CHANGELOG.md) ·
[Development](https://github.com/paultuckey/typed-handlebars/blob/main/Development.md)

## Licence

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT licence ([LICENSE-MIT](LICENSE-MIT))

at your option. This is the Rust ecosystem convention: the MIT licence is the permissive default,
and Apache-2.0 adds an explicit patent grant that some downstream users need.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this work by you, as defined in the Apache-2.0 licence, shall be dual-licensed as above, without
any additional terms or conditions.

The Handlebars parser is derived from
[rusty-handlebars](https://github.com/h-i-v-e/rusty-handlebars) — see [NOTICE](NOTICE) for
attribution.

