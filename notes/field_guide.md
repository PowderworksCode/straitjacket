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
- `install.sh`, `action.yml` and `Cargo.toml` carry
  `straitjacket-allow-file:no-comments`. None of `sh`, YAML or TOML has a
  documentation-comment syntax to hoist reasoning into, and all three are
  interface that people read before trusting. That is the reason; do not extend
  the marker to implementation source.
- `blake3` is a direct dependency that nothing calls. It is there to force its
  `pure` feature across the graph: wasmer pulls blake3, whose `build.rs`
  compiles NEON assembly with `cc` on aarch64 and so wants
  `aarch64-linux-musl-gcc`. x86_64 uses intrinsics and builds fine, so removing
  the entry breaks the aarch64 release and nothing else — which CI does not
  build. Do not tidy it away.
- CI builds the musl release target on every run. A release build that only
  breaks when a tag is pushed is discovered too late to fix quietly.
- `install-smoke.yml` is **called** by the release workflow, not triggered by
  `release: published`. An event raised by `GITHUB_TOKEN` does not start
  another workflow run, so a `release` trigger there fires never. This was not
  theoretical: it silently did nothing on the first v0.1.0 release.
- A GitHub expression treats the empty string as falsy, so `x && '' || y`
  yields `y`, not the empty string. Clearing a value conditionally has to
  happen in the shell step. Both `action.yml` and `install-smoke.yml` were
  written wrong here once.
- `install.sh` downloads through the crates.io-style API asset endpoint when a
  token is present, because the plain release download URL redirects to another
  host and curl drops the credential across the hop. Without that, the script
  cannot install from a private repository at all.
- Updating is re-running the installer. It has no `self update` subcommand and
  no version check: the installer already resolves the latest release, verifies
  it, and replaces the binary by rename, and the alternative is a CI gate that
  makes network requests of its own. Adding one would mean an HTTP client, TLS,
  gzip and sha256 in a crate whose whole dependency set is nine lean crates, in
  a binary whose job is to run in other people's CI. Linters do not ship
  self-update; toolchain managers do, and this is not one.
- `action.yml` resolves an unset `version` input to the release the Action ref
  ships, by reading the `Cargo.toml` sitting beside it in that checkout. That is
  what makes `@v0.1.3` pin the scanner and not just the wrapper, and it is the
  whole update strategy for CI: a bump is one line, and the new release's
  findings land in the pull request that bumps it. A ref naming a version with
  no release falls back to `latest` with a warning annotation — that case is
  `@main`, a fork, and this repository's own `uses: ./` job during the window
  between the commit that bumps `Cargo.toml` and the tag that releases it. An
  explicitly requested `version` never falls back: it was chosen on purpose.
- `scripts/install.sh` is **fleet-owned**, from
  `conf/.ordnung/managed/publishing/rust/install.sh` with `{{name}}`,
  `{{NAME}}`, `{{repo}}` and `{{website}}` substituted, as are
  `.github/workflows/release.yml`, `install-smoke.yml` and `.cargo/config.toml`.
  Improving the installer — having it report the version it replaced, say — is
  an edit in `conf`, and doing it here is drift the next sync overwrites.
  `action.yml` is **not** managed: ordnung carries a different one.

## Publishing to crates.io

- `Cargo.toml` excludes `assets/*`. The README banner is for the repository
  page; shipping half a megabyte of JPEG to everyone who depends on the crate
  is not. `cargo package --list` is the check.

- The publish logic lives in `scripts/publish.sh`, not in the workflow, so the
  one irreversible action in this repository can be run and read by hand. The
  `crate` job is a caller. `scripts/publish.sh --dry-run` also runs in CI on
  every change, so the publish path is exercised long before the tag that
  makes it real.
- `--index` points the already-published check at something other than
  index.crates.io, which is what would let the whole path be rehearsed against
  a local sparse index.
- The release workflow takes a `concurrency` group with
  `cancel-in-progress: false`. A cancelled `cargo publish` can leave a version
  uploaded that can never be reused.
- The `crate` job names the `crates-io` environment, which is where a required
  reviewer goes if a publish should need a human first.
- The `crate` job in the release workflow publishes with **trusted
  publishing**: crates.io mints a token over OIDC that lives under an hour and
  the action revokes it at the end. No registry token is stored in this
  repository, and none should ever be.
- crates.io scopes that trust to this repository **and this workflow filename**.
  Renaming `release.yml` breaks publishing until the trusted publisher is
  updated on crates.io to match.
- The job runs after the install smoke test, because a crates.io version can
  never be replaced, only yanked. Everything that can be checked is checked
  before the irreversible step.
- It skips a version crates.io already has, so re-running a release for an
  existing tag is safe.
- The **first** publish of a crate cannot use trusted publishing. crates.io has
  no equivalent of PyPI's pending publishers, so a new crate must be published
  once with an API token before the trusted publisher can be configured
  against it. A `CARGO_REGISTRY_TOKEN` repository secret covers exactly that
  one release: when it is set the job uses it, and when it is absent the job
  uses OIDC. Delete the secret once the trusted publisher exists — leaving it
  there defeats the reason for any of this.

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
- The fleet treats a mutable tag as a supply-chain hole, which is why Action
  pins here are 40-character commit SHAs. That rules out publishing a moving
  `v0` or `v0.1` tag for consumers of this repository's Action, however
  conventional one is elsewhere. Consumers pin an exact tag or a SHA, and
  Dependabot moves it.
- CI is one `gate` job: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`. Actions are pinned by
  commit SHA; do not swap one for a tag to make updating easier.
- The toolchain comes from `rust-toolchain.toml`, not from a setup action.
