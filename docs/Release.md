# Release

Releasing is manual and local, driven by [`cargo release`](https://github.com/crate-ci/cargo-release):

```shell
cargo install cargo-release
```

One command does the whole sequence — bump the version in both places it appears, rewrite the
changelog, commit, tag, publish `typed-handlebars-macros` and then `typed-handlebars`, and push.
The settings live in [`release.toml`](../release.toml),
[`typed-handlebars/release.toml`](../typed-handlebars/release.toml), and a
`[package.metadata.release]` opt-out in `example/Cargo.toml`.

`cargo publish` is irreversible — a version can be yanked, never replaced — so run the dry run and
the checks first.

## 1. Write the changelog

Add entries under `## [Unreleased]` in `CHANGELOG.md`. The release turns that heading into the
version being released, dates it, and re-points the links at the bottom; nothing there needs
editing by hand.

## 2. Run the checks

```shell
cargo fmt --check && cargo clippy --workspace --all-targets && cargo test --workspace
```

The README example, built as an outside crate:

```shell
cd consumer-test && cargo run
```

The handlebars.js parity suite:

```shell
cd reference-ts && npm install && npm test
```

## 3. Dry run

`major`, `minor` or `patch` — or an exact version. Without `--execute` nothing is written,
committed, pushed or uploaded:

```shell
cargo release minor
```

Read the diff it prints for the changelog and the manifests. One line in the output is expected
noise: `depends on unpublished workspace package typed-handlebars-macros` — the macros crate is
never really uploaded in a dry run, so verifying the crate that depends on it cannot resolve the
new version. A real run publishes macros first and waits for the index.

## 4. Release

Needs a crates.io token — `cargo login` once, or `CARGO_REGISTRY_TOKEN` in the environment. The
same level as the dry run, plus `--execute`:

```shell
cargo release minor --execute
```

It asks for confirmation before anything irreversible.

## 5. Write the GitHub release

Create a release against the new `v…` tag on
[GitHub](https://github.com/paultuckey/typed-handlebars/releases/new), with the changelog entry as
its body.
