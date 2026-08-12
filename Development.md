
## Development

```shell
cargo fmt
```

```shell
cargo clippy
```

```shell
cargo test
```

`reference-ts/` runs the same templates through real handlebars.js, so the supported subset above is
checked against the language it claims to implement rather than against itself:

```shell
cd reference-ts && npm install && npm test
```

`typed-handlebars/tests/ui/` holds compile-fail tests pinning the error messages a developer sees when
a template or its wiring is wrong. After changing a diagnostic on purpose, regenerate the expected
output and read the diff:

```shell
TRYBUILD=overwrite cargo test -p typed-handlebars --test ui
```
