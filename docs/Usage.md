# Usage Notes

## Composing and writing

A template implements `Display`, so a nested one can be passed straight to its parent and written
once into the same buffer — no `String` per level:

```rust
templates::page(templates::row("King")).render()   // page.hbs writes {{{ rows }}}
```

`render_to` writes into any `fmt::Write` sink, so a response buffer needs no throwaway `String`:

```rust
let mut body = String::from("<body>");
templates::page(rows).render_to(&mut body)?;
```

`render()` remains the convenience form and returns a `String`.


### Absent values

A variable can be an `Option`, and `None` renders as nothing — as null and undefined do in
handlebars.js. Nothing in the template says so and nothing at the call site unwraps first, which
matters because most database columns are nullable:

```rust
templates::row(row.id, row.guessed_datetime).render()   // Option<String>, None renders as ""
```

This works by value or by reference, in a record, in `{{#each}}`, and through a builder setter.
`{{#if}}` asks only whether the value is there, so `{{#if x}}{{ x }}{{/if}}` works on the very
`Option` it prints. `false` and `0` are values rather than absences, and render as `false` and `0`.


## Names Rust would object to

Templates and variables are named by whoever writes the `.hbs` files, so names Rust reserves are
renamed rather than rejected: a keyword gets a trailing underscore and a leading digit gets a
leading one.

```
{{ type }}        .type_(…)
mod.hbs           templates::mod_(…)
2col.hbs          templates::_2col(…)
```

Your IDE offers the renamed form, so in practice you meet it through autocomplete.


## When a template is wrong

Mistakes in a `.hbs` file are reported against the file, with a line and column, in Handlebars
terms:

```
error: templates/results.hbs:2:6: `{{#each}}` is never closed — it needs a matching `{{/each}}`
```

Constructs that are not supported yet say so by name rather than turning into a Rust error. With
`directory!`, one broken template does not stop the others compiling.
