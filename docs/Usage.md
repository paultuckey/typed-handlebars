# Usage Notes

## The two ways in

`Vars` is every variable the template uses, written as an ordinary struct literal. It is exhaustive
on purpose: the compiler names anything you misspell or forget, so a variable added to the `.hbs`
breaks the call sites instead of quietly rendering as nothing.

```rust
templates::page::Vars { title: "Dub", rows: &rows }.render()
```

`builder()` is for when you do not have every variable. Handlebars renders an undefined variable as
nothing, and a struct literal has no way to leave a field out, so this is the form that expresses it:

```rust
templates::page::builder().title("Dub").render()   // rows renders as no items
```

Nested types work the same way: a `{{#each}}` item is a plain struct, with `builder()` hanging off it
for the partial case.

```rust
templates::page::RowsItem { id: 1, name: "King" }
templates::page::RowsItem::builder().name("King").build()
```


## Composing and writing

`render_to` writes into any `fmt::Write` sink, so a response buffer needs no throwaway `String`:

```rust
let mut body = String::from("<body>");
templates::page::Vars { title: "Dub", rows: &rows }.render_to(&mut body)?;
```

`render()` is the convenience form and returns a `String`.

One template's output goes inside another through `{{{ }}}`, which is how handlebars.js composes
too — render the inner one and pass the markup in as a variable:

```rust
templates::page::Vars { content: templates::row::Vars { name: "King" }.render() }.render()
```

Unlike `{{> partial}}`, which is resolved at compile time, this lets the content be chosen at run
time — a layout wrapping whichever page the request asked for.


### Absent values

A variable can be an `Option`, and `None` renders as nothing — as null and undefined do in
handlebars.js. Nothing in the template says so and nothing at the call site unwraps first, which
matters because most database columns are nullable:

```rust
// Option<String>, None renders as ""
templates::row::Vars { id: row.id, seen: row.guessed_datetime }.render()
```

A bare `None` has no type to infer, so give it one — a typed binding, or `None::<&str>` — or leave
the variable out through `builder()`, which is usually what you meant.

This works by value or by reference, in a record, in `{{#each}}`, and through a builder setter.
`{{#if}}` asks only whether the value is there, so `{{#if x}}{{ x }}{{/if}}` works on the very
`Option` it prints. `false` and `0` are values rather than absences, and render as `false` and `0`.


## Naming a generated type

Most call sites let inference produce the template's type and never name it. Once an application has
more than one call site for the same template, it usually wants the mapping from its own types in one
place — which means writing the type down:

```rust
fn todo_item(todo: &Todo) -> todo::Vars<i64, bool, &String> {
    todo::Vars { id: todo.id, done: todo.done, title: &todo.title }
}

impl<'a> From<&'a Todo> for todo::Vars<i64, bool, &'a String> { /* … */ }
```

One parameter per field, in the order the template first mentions them, and nothing else — a value
that is written goes through a `Render` marker saying how, but the markers live on `render` as
method-level generics, so they never appear in a signature. `Option` included:

```rust
fn pair(b: Option<u32>) -> maybe::Vars<&'static str, Option<u32>>
```

Rename a variable in the `.hbs` and you get one compile error, in the function that owns the
mapping, rather than one per call site. The value can also be stored — in a struct field, in a `Vec`
— and rendered more than once, which is what makes it different from a rendered `String`.

A list is where the parameters stop threading up. Its item type is named only in an `AsRef` bound,
never in a field, so the container parameter stays opaque and the item's own parameters live inside
it:

```rust
deep::RowsItem<&str, Vec<deep::RowsItemCellsItem<i32>>>   // two, not three
```

That is also why no generated type needs a `PhantomData`, and so why all of them can be written as
literals.


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
templates::page::Vars { rows }.render()   // page.hbs writes {{ rows.length }} and {{#each rows}}
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
{{ type }}        Vars { type_: … }
mod.hbs           templates::mod_::Vars
2col.hbs          templates::_2col::Vars
```

The same escape settles a collision between two generated names. `{{ builder.name }}` is ordinary
Handlebars, but `builder` camel-cases onto the builder this crate generates, so one of them has to
give way:

```
{{ builder.x }}   the record becomes Builder_, since Builder is the module's own
{{ vars.x }}      likewise Vars_
```

Names are handed out in the order the template mentions them, and the module's own API — `Vars`,
`Builder`, `builder()` — is reserved ahead of all of them, so those always mean what a caller
expects. Your IDE offers the renamed form, so in practice you meet it through autocomplete.


## When a template is wrong

Mistakes in a `.hbs` file are reported against the file, with a line and column, in Handlebars
terms:

```
error: templates/results.hbs:2:6: `{{#each}}` is never closed — it needs a matching `{{/each}}`
```

Constructs that are not supported yet say so by name rather than turning into a Rust error. With
`directory!`, one broken template does not stop the others compiling.
