# Changelog

All notable changes to Straitjacket are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `site/` — the source for [straitjacket.dev](https://straitjacket.dev), moved
  into this repository so the documentation and the tool version together.
- Crate metadata for discovery on crates.io: `homepage`, `documentation`,
  `keywords`, and `categories`.
- `CHANGELOG.md` and `CONTRIBUTING.md`.
- A machine-readable rule manifest: `straitjacket --list-rules --format json`
  emits every rule, every withdrawn rule, and the tunable defaults.
  `scripts/rules-manifest.sh` exports it to `site/content/rules.json`, and a
  `docs` workflow fails the build when the manifest is stale, when a rule is
  undocumented, when a page names a rule the binary does not have, or when a
  page quotes a default that has moved.

### Changed

- One description everywhere. The crate, the repository, `--help`, the README
  and the site had drifted into five different pitches; they now all say the
  tool flags the weird code and text LLMs produce.
- Documentation now matches the shipped rule set. The site described
  `slop-prose`, `duplication`, `one-component`, `effect-in-component`,
  `prop-drilling` and `store-passthrough`, none of which exist in this
  repository, and documented a `.straitjacket.yaml` config file that the binary
  silently ignored. The real file is `straitjacket.toml`.

### Fixed

- Every GitHub link on the website pointed at `zmaril/straitjacket`, which is
  private and returns 404 to visitors — including the install command on the
  front page, so the advertised quick start could not work for anyone.
- Documented defaults that did not match the binary: the `deep-nesting` budget
  is 8, not 6; there is no prebuilt Windows binary; the installer writes to
  `~/.local/bin`, not `/usr/local/bin`; unknown rule ids are a hard error, not
  a warning.
- The crate no longer packages `site/` and `notes/`.

## [0.1.1] - 2026-08-09

### Changed

- Named the GitHub Action "Powderworks Straitjacket" so it can be listed on the
  Marketplace.
- Rewrote the README around the reference material: rules, configuration,
  suppression, and the Action's inputs.

## [0.1.0] - 2026-08-09

First public release. A single static binary, published to crates.io with
prebuilt archives for Linux (`x86_64`, `aarch64`, static musl) and macOS
(`arm64`, `x86_64`).

### Added

- Ten lexical rules: `color`, `deep-nesting`, `emoji`, `file-size`,
  `inline-font`, `inline-svg`, `motion`, `stray-todo`, `unused-marker`, and the
  opt-in `no-comments`.
- `text`, `json` and SARIF 2.1.0 output, the last for GitHub code scanning.
- Line- and file-scoped suppression markers, with `unused-marker` reporting the
  ones that stopped suppressing anything.
- `straitjacket.toml` configuration, discovered from the working directory
  upward.
- `straitjacket instructions`, which prints the repository's policy as prose for
  a `CLAUDE.md`, an `AGENTS.md`, or an agent hook.
- A composite GitHub Action, and an `install.sh` that verifies the download
  against the release checksums.

### Removed

- The eight rules backed by unpublished `entl`/`infact` packs — `exact-clone`,
  `near-clone`, `library-opportunity`, `effect-barrier`, `effect-capability`,
  `unknown-barrier`, `error-discard` and `analysis-incomplete`. Cutting them is
  what let Straitjacket build on its own. A `straitjacket.toml` still carrying a
  `[facts]`, `[effects]` or `[errors]` section is rejected with an error naming
  the rules that went away.

[Unreleased]: https://github.com/PowderworksCode/straitjacket/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/PowderworksCode/straitjacket/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/PowderworksCode/straitjacket/releases/tag/v0.1.0
