# Usage Notes

### Composing and writing

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
