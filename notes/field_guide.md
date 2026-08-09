# Agent Field Guide

Read this before changing the repository. Add concise entries when work reveals
a durable constraint, a non-obvious convention, or a recurring failure mode that
would help a future agent. Keep temporary plans and task-specific notes out.

## Standing on its own

- Straitjacket depends only on published crates. It previously took Cargo `path`
  dependencies on sibling `entl` and `infact` checkouts, which meant it could
  not build anywhere except a full `powderworks/` tree. Do not reintroduce a
  path dependency; if something here needs a sibling crate, that is a signal the
  feature belongs in the sibling, not here.
- `src/language.rs` is the whole language model: extensions, filenames,
  shebangs, comment syntax, and the facets rules select on. It replaced
  `entl-codebase`. Adding a language is an entry in the table, and a test
  asserts no two languages claim the same extension.
- `src/walk.rs` is the file walk, over the `ignore` crate. A walk error is
  returned rather than skipped, because a scan over less than the requested tree
  must not be able to report clean.

## Distribution

- `.cargo/config.toml` links both `*-unknown-linux-musl` targets with `rust-lld`
  against a self-contained musl. That is what lets `cargo build --release
  --target aarch64-unknown-linux-musl` cross-compile from an ordinary x86_64
  host with no cross toolchain, container, or apt package, and it is why the
  release workflow is one plain `cargo build` per target. Reproduce a release
  binary locally with exactly that command.
- The release workflow creates a **draft**, uploads every archive, computes
  `SHA256SUMS` from what the release actually holds, and only then publishes.
  A draft is not returned by the `releases/latest` endpoint, so `install.sh`
  can never resolve a half-uploaded release. Do not reorder those steps.
- The asset name `straitjacket-<tag>-<target>.tar.gz` is a contract between the
  release workflow and `install.sh`. Changing it in one place breaks every
  existing installer invocation.
- `install.sh` and `action.yml` carry `straitjacket-allow-file:no-comments`.
  Neither `sh` nor YAML has a documentation-comment syntax to hoist reasoning
  into, and both files are interface that people read before trusting. That is
  the reason; do not extend the marker to implementation source.
- CI builds the musl release target on every run. A release build that only
  breaks when a tag is pushed is discovered too late to fix quietly.

## Rules

- Every rule is lexical. Nothing parses, resolves a type, or follows a call.
  Rules that needed more than that were removed rather than weakened, and
  `src/rules/mod.rs::REMOVED` lists them so an old configuration fails saying
  the rule was withdrawn instead of saying the key is unknown.
- `color`, `inline-font`, `inline-svg`, and `motion` are `RegexRule`s gated on a
  language facet (`STYLE_HOST`, `COMPONENT_HOST`). A language without the facet
  correctly produces nothing: a hex string in Python is not a CSS color, and
  giving Python a style facet would make every hex string in every Python file a
  finding.
- Rules self-register through `inventory`, so no core file names a rule. Adding
  or deleting one touches only its own file and `src/rules/mod.rs`.

## Its own policy

- Straitjacket runs its full ruleset on itself and is clean. `no-comments` is ON
  here, which is worth knowing before adding code: an ordinary `//` inside a
  function body is a finding. Explanations go in doc comments, and test
  commentary goes in assert messages.

## Fleet

- `.github/dependabot.yml` is fleet-owned and comes from the `conf` repository.
  Editing it here is drift, and the next sync overwrites it.
- CI is one `gate` job: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`. Actions are pinned by
  commit SHA; do not swap one for a tag to make updating easier.
- The toolchain comes from `rust-toolchain.toml`, not from a setup action.
