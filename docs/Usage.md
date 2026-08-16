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


## Naming a generated type

Most call sites let inference produce the template's type and never name it. Once an application has
more than one call site for the same template, it usually wants the mapping from its own types in one
place — which means writing the type down:

```rust
fn todo_item(todo: &Todo) -> todo::Template<i64, bool, &String> {
    todo::Template::new(todo.id, todo.done, &todo.title)
}

impl<'a> From<&'a Todo> for todo::Template<i64, bool, &'a String> { /* … */ }
```

One parameter per variable, in the order the template first mentions them. Rename a variable in the
`.hbs` and you get one compile error, in the function that owns the mapping, rather than one per call
site. The value can also be stored — in a struct field, in a `Vec` — which is what makes this
different from erasing it behind `impl Display`.

A written value carries a hidden marker parameter saying how it renders, but markers are declared
last and default to `ViaDisplay`, so they can be left off. `Option` is the exception: its marker is
`ViaOption` (or `ViaOptionRef` by reference), and because Rust only elides defaults from the right,
every marker before it has to be spelled too:

```rust
fn pair(b: Option<u32>) -> maybe::Template<&'static str, Option<u32>, ViaDisplay, ViaOption>
```

Templates with no `Option` leaves elide all of them.


## Reaching the top level

`{{@root.title}}` reads the template's own top-level context from any depth — inside `{{#each}}`,
inside `{{#with}}`, or nested in both:

```handlebars
{{#each rows}}<li>{{ name }} — {{@root.title}}</li>{{/each}}
```

`@root` is absolute, unlike `{{@index}}`, `{{@first}}` and `{{@last}}`, which are loop state:
`{{@../root.title}}` means exactly the same thing, as it does in handlebars.js. It works as a block
subject too (`{{#each @root.rows}}`, `{{#with @root.person}}`), and a partial sees the including
template's root, since a partial renders against the context it was included from.

`{{@root}}` on its own is a compile error — handlebars.js writes `[object Object]` for the whole
context, and there is nothing useful to write instead.


## Counting a list

`{{ rows.length }}` counts the list, and the same variable can still be iterated:

```rust
templates::page(rows).render()   // page.hbs writes {{ rows.length }} and {{#each rows}}
```

Anything slice-backed counts — `Vec`, an array, a slice, or a reference to one. A `String` does
not: JS counts UTF-16 code units where Rust counts bytes or `char`s, so it is a compile error
naming the value rather than a quietly different number. A record field genuinely named `length`
is unreachable, because `.length` always means the count.

A list left unset on a builder counts as nothing rather than `0`, as an undefined value does in
handlebars.js. A list you pass with no items in it counts `0`.


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
