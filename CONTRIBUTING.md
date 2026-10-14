# Contributing

## Branches

- `main` holds released code only. Every commit on it is a version.
- `dev` is where work comes together. Branch from it, and open pull requests into it.
- Name branches after the change: `feat/journal-chain`, `fix/jump-rekey`.

Pull requests into `dev` are squash-merged, so the pull request title becomes the commit.

## Commits

[Conventional Commits](https://www.conventionalcommits.org), one line, no body.

```text
feat(journal): chain events with hmac-sha256
fix(ssh): keep the jump channel open during rekey
ci: cache the cargo registry
```

- Types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `build`, `ci`, `chore`, `style`, `revert`.
- Scope: the crate without the `synt-` prefix, `mcp`, or nothing.
- Subject: imperative, starts lowercase, no period, the whole line at most 72 characters.
- A breaking change gets a `!`: `feat(proto)!: rename action.call`.

`git config core.hooksPath .githooks` turns on the local check.

## Code

- One main type per file, named after it. A concept with several files gets its own folder.
- Private by default. `pub` only for what the crate offers to others.
- Errors with `thiserror`, one enum per crate. No `unwrap()` outside tests.
- Full words in names: `connection`, not `conn`.

## Shape of code

- No comments and no doc comments. Names, types and small functions carry the meaning.
- A blank line between logical steps; intermediate results get names.
- Guards first, the main path flat below.
- Imports grouped `std`, external crates, `crate::`.

## Before you push

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same commands on macOS, Windows and Linux, plus `cargo deny check` for dependency licenses.

## Fuzzing

Every parser that reads bytes from outside has a target in `fuzz/`. They need nightly and `cargo install cargo-fuzz`:

```bash
cargo +nightly fuzz list
cargo +nightly fuzz run osc133_scanner -- -max_total_time=180
```

A crash is a bug: fix it and keep the input as a regression test in the crate.
