# Agent Field Guide

Read this before changing the repository. Add concise entries when work reveals
a durable constraint, a non-obvious convention, or a recurring failure mode that
would help a future agent. Keep temporary plans and task-specific notes out.

## The widest dependency surface in the fleet

- Straitjacket builds both Entl and infact from source, through Cargo `path`
  dependencies on sibling checkouts. CI therefore checks out three repositories
  side by side, so that `../entl` and `../infact` resolve the way they do in a
  local `powderworks/` checkout:

  ```
  $GITHUB_WORKSPACE/straitjacket
  $GITHUB_WORKSPACE/entl
  $GITHUB_WORKSPACE/infact
  ```

- Both sibling checkouts are **unpinned** — they track those repositories'
  default branches. A change in either can break this build with nothing landing
  here, and the weekly scheduled run exists partly to surface that between pull
  requests. When CI fails and nothing here changed, check what moved in Entl
  and infact first.
- `Cargo.lock` picks up crates that appear in the siblings, so a lockfile-only
  diff here is usually the sibling growing a crate rather than a change of
  intent.

## Analysis

- Clone detection runs through Entl's `parse_repository`, which walks the whole
  repository, not just the files a rule targets.
- `exact-clones` is off in this repository's own `straitjacket.toml`. Turning it
  on means every recognized language in the tree is parsed, so a language with
  no parser pack surfaces as `analysis-incomplete`. That is how a missing pack
  becomes a test failure here rather than in Entl.
- `parser-paths` in the config points at `../entl/parser-packs`. Tests that
  construct a config write that path explicitly, so they depend on the sibling
  checkout existing.

## Fleet

- `.github/dependabot.yml` is fleet-owned and comes from the `conf` repository.
  Editing it here is drift, and the next sync overwrites it.
- CI is one `gate` job: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`. Actions are pinned by
  commit SHA; do not swap one for a tag to make updating easier.
- The toolchain comes from `rust-toolchain.toml`, not from a setup action.
