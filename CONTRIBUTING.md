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

## SSH integration tests

The tests in `crates/ssh/tests/servers` and `crates/core/tests/servers` talk to real OpenSSH servers. They are ignored by default and skip themselves unless `SYNT_SSH_TESTS=1` is set.

```bash
tests/ssh/up.sh
SYNT_SSH_TESTS=1 cargo nextest run -p synt-ssh -p synt-core --test servers --run-ignored only
tests/ssh/down.sh
```

`up.sh` needs Docker and `ssh-keygen`. It generates fresh client and host keys in `tests/ssh/run/` (ignored by git) and starts two Alpine OpenSSH containers: `bastion`, published on a random port of `127.0.0.1`, and `target`, reachable only from the bastion as `target:22`. Both log in user `synt` with the client key or the vault key and run a TCP echo on port 7000. The tests write their own `known_hosts` into a temporary folder and never touch `~/.ssh`. CI runs them in the `ssh-integration` job.

## Extension tests

`tests/extension/everything` is an extension built with the [extension SDK](https://github.com/syntso/extension-api) that uses every capability and contribution point of API 0.2. Its build, `tests/extension/everything.wasm`, is committed, so `cargo test --workspace` needs no WebAssembly toolchain. After changing the fixture, build it and commit the new `.wasm`:

```bash
rustup target add wasm32-wasip2
cargo build --manifest-path tests/extension/everything/Cargo.toml --target wasm32-wasip2 --release
cp tests/extension/everything/target/wasm32-wasip2/release/everything.wasm tests/extension/
```

`SYNT_EVERYTHING_WASM=<path>` makes the tests load another build instead. CI's `extension-fixture` job rebuilds the fixture from source and runs `crates/extensions/tests/{api,install}.rs` and `crates/core/tests/everything` against it.

## Kubernetes integration tests

The tests in `crates/kube/tests/cluster` and `crates/core/tests/clusters` talk to a real cluster. They are ignored by default and skip themselves unless `SYNT_KUBE_TESTS=1` is set.

```bash
tests/kube/up.sh
SYNT_KUBE_TESTS=1 cargo nextest run -p synt-kube --test cluster -p synt-core --test clusters --run-ignored only
tests/kube/down.sh
```

`up.sh` needs Docker and `ssh-keygen`. It starts a single-node k3s cluster (`rancher/k3s`, the image k3d runs) with its API server published on a random port of `127.0.0.1`, and an OpenSSH `bastion` from `tests/ssh` on the same Docker network, where the API server is also reachable as `cluster:6443`. It writes `tests/kube/run/kubeconfig` (ignored by git) with three contexts: `direct` (the published port, client certificate), `inside` (`https://cluster:6443`, reachable only through the bastion, so it checks the SSH tunnel and the TLS name) and `plugin` (the published port with an exec credential plugin that prints a service account token). The tests make a namespace of their own and delete it at the end; pods run `busybox`. CI runs them in the `kube-integration` job.
