# dry-handlebars

_Experimental_ compile-time checked [Handlebars](https://handlebarsjs.com/) templates for Rust. Based on the parser
from [rusty-handlebars](https://github.com/h-i-v-e/rusty-handlebars).

The blog post [code first or schema first](https://blog.logrocket.com/code-first-vs-schema-first-development-graphql/)
highlights that there are two way of thinking about templating. Code first or template first.

This library takes a template first approach. The designer makes pure handlebars files (hbs) that can be edited
separately. Then the Rust developer gets a pure rust experience with compile time checking of templates and how they are
called from Rust.

The Rust developer should not have to repeat the template name or variable names in Rust code, hence the name DRY (don't
repeat yourself).

## Goals

**As much as possible at compile time, as little as possible at runtime.** Templates are turned into Rust when the crate
is built, so there is no parsing, no template registry and no lookups while your program runs — just the code the
template implies. Where a design choice trades build-time work against run-time work, build time wins.

**The Handlebars author needs to know no Rust.** An `.hbs` file is plain Handlebars, written by someone who never has to
think about what happens downstream. No Rust type names, no annotations, no macro-specific syntax — nothing in the
template that a designer could not write, or that would stop the same file rendering under handlebars.js.

**The code generator takes on the complexity.** The template already says what data it needs: `{{#each rows}}{{ name }}`
means a list of records with a `name`. The macro reads that and generates the types, so nobody has to declare them
twice. There are no traits for you to implement and nothing to derive.

**The Rust developer only does the wiring.** Connect your data to what the generator produced, with the names supplied
by IDE autocomplete rather than retyped from the template. Getting a name wrong should be a compile error, not something
you discover in a rendered page.

Example: Take a directory of handlebars files:

`templates/button.hbs`:

```handlebars
<button id="btn{{ btn_id }}" class="btn btn-light">
    {{ btn_name }}
</button>
```

Then in rust:

```rust
mod templates {
    dry_handlebars::directory!("templates/");
}
fn get_html() -> String {
    // templates::button is automatically generated 
    templates::button(42, "Save").render()
}
```

### Optionally, a builder

Every template also gets a builder. It is entirely optional — the function above is the normal way to
call a template — but it names each variable, so nothing depends on argument order and your IDE
offers the names rather than you retyping them:

```rust
templates::button_builder::new()
    .btn_id(42)
    .btn_name("Save")
    .render()
```

You only set what you have. Anything you leave out renders as empty, a list with no items, or a false
condition, exactly as an undefined variable does in Handlebars:

```rust
templates::button_builder::new().btn_id(42).render()   // btn_name renders as nothing
```


## What is supported

Still alpha. The table below is the whole of it — anything not listed is a compile error naming the
construct, never a silent difference and never a Rust type error you would have to decode.

### Works

| Construct                                   | Notes                                                                   |
|---------------------------------------------|-------------------------------------------------------------------------|
| `{{ name }}`                                | HTML-escaped                                                            |
| `{{{ name }}}`                              | raw, for markup you have already rendered                               |
| `{{ person.name }}`                         | a `person` record is generated                                          |
| `{{ ../name }}`                             | reaches the enclosing scope                                             |
| `{{#if}}` / `{{#unless}}` / `{{else}}`      | Handlebars truthiness; testing a variable does not stop you printing it |
| `{{#each rows}}`                            | with `{{this}}`, `{{@index}}`, `{{else}}`, and `as \|row\|`             |
| `{{#with person}}`                          | see the divergence below                                                |
| `{{> row}}`                                 | partials, rendered against the context they were included from          |
| `{{! … }}` / `{{!-- … --}}`                 | comments                                                                |
| `{{~ … ~}}`                                 | whitespace trimming                                                     |
| `\{{ … }}` and `{{{{raw}}}} … {{{{/raw}}}}` | literal output                                                          |

### Not yet

`{{else if}}` · `{{@key}}` `{{@value}}` `{{@first}}` `{{@last}}` `{{@root}}` · `{{lookup}}` ·
sub-expressions `( … )` · `{{#with}}` with `{{else}}` · partial arguments (`{{> row this}}`) ·
inline partials (`{{#*inline}}`) · `{{!-- … --~}}` · lists that are not slice-backed (`HashMap`,
`VecDeque`) · handlebars.js's standalone-partial indentation.

### Out of scope

**Helpers** A helper is Rust code, and a template that needs Rust code stops
being something the designer can own. `{{myhelper x}}` and `{{log}}` are compile errors naming the
helper. Anything a helper would have done belongs in the wiring.

**Runtime template loading.** Templates are compiled into your binary, so there is nothing to load
and no dynamic partial names.

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

Partials are resolved at compile time by splicing, so there is no second `render` call and no
intermediate `String`. Editing a partial rebuilds every template that includes it. Cycles, unknown
names and arguments (`{{> row this}}`, not supported yet) are all compile errors. Partials need a
directory to look in, so they work with `directory!` and `file!` but not `str!`.

### Escaping

`{{ name }}` HTML-escapes its value and `{{{ name }}}` does not, as Handlebars specifies. Escaping
covers the same characters handlebars.js escapes - `&`, `<`, `>`, `"`, `'`, `` ` `` and `=` - and
happens as the value is written, so nothing is allocated for it.


### When a template is wrong

Mistakes in a `.hbs` file are reported against the file, with a line and column, in Handlebars
terms:

```
error: templates/results.hbs:2:6: `{{#each}}` is never closed — it needs a matching `{{/each}}`
```

Constructs that are not supported yet say so by name rather than turning into a Rust error. With
`directory!`, one broken template does not stop the others compiling.

### Known divergence from handlebars.js

`{{#with person}}` renders its block even when `person` was never set, showing empty fields, where handlebars.js would
skip the block. `{{#if}}` and `{{#unless}}` are unaffected — an absent variable is correctly falsy there.

