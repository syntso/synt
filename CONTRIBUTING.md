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
- Scope: the crate without the `synt-` prefix, `app`, `mcp`, or nothing.
- Subject: imperative, starts lowercase, no period, the whole line at most 72 characters.
- A breaking change gets a `!`: `feat(proto)!: rename action.call`.

`git config core.hooksPath .githooks` turns on the local check.

## Code

- One main type per file, named after it. A concept with several files gets its own folder.
- Private by default. `pub` only for what the crate offers to others.
- Errors with `thiserror`, one enum per crate. No `unwrap()` outside tests.
- Full words in names: `connection`, not `conn`.

## Comments

- Every public item has a `///` doc comment. The first line says what it is or does.
- `# Errors`, `# Panics` and `# Safety` where they apply. Key types get an `# Examples` that runs as a test.
- An inline `//` explains why, when the code cannot. It never retells what the code does.
- No commented-out code, no `TODO` without an issue number.

## Before you push

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same commands, plus `cargo deny check` for dependency licenses.
