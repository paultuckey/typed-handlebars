### Development

```shell
cargo fmt 
```

```shell
cargo clippy 
```

```shell
cargo test 
```

`dry-handlebars/tests/ui/` holds compile-fail tests that pin the error messages a developer sees when
they get the wiring wrong. After changing a diagnostic on purpose, regenerate the expected output and
read the diff:

```shell
TRYBUILD=overwrite cargo test -p dry-handlebars --test ui
```