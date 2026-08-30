# Changelog

All notable changes to Straitjacket are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- [Update Straitjacket](https://straitjacket.dev/guides/updating), a guide for
  the thing every installed tool eventually needs and this one never documented:
  re-run the installer, roll back with `STRAITJACKET_VERSION`, bump the Action
  tag, and keep a machine on the release CI runs. Nothing checks for a new
  version behind your back, and the guide says so -- for a gate, an update that
  arrives on its own is a red build nobody caused.

### Changed

- The Action's tag pins the scanner. `version` now defaults to the release the
  Action ref ships, read from the `Cargo.toml` beside `action.yml` in that ref,
  rather than to `latest` -- so `@v0.1.3` runs v0.1.3 and a release published
  afterwards cannot apply new rules to a branch that changed nothing. That is
  the update strategy this project wants: a bump is a line in a workflow, and
  the findings it turns up arrive in the pull request that bumps it. `latest`
  is still available by name, and `version` still overrides. A ref that is not
  a release -- `@main`, or a version bumped in `Cargo.toml` before its tag is
  pushed -- installs the latest release and says so in a warning annotation,
  which is what keeps this repository's own `uses: ./` job working between a
  version bump and its tag.

- The site no longer names the release it documents. Every page that hands a
  reader a tag to paste writes `{{version}}`, and the build fills it from
  `Cargo.toml` — so a page cannot fall behind a release, and nothing has to be
  run or remembered to keep it current. This needs `powderworks-docs` with
  `--var`, which the lockfile now takes.

### Fixed

- The site and the README name the current release. They said `v0.1.1`, two
  releases after that stopped being true, so the CI guide told readers to pin
  a scanner three versions behind the page describing it. The README is the one
  tag still written by hand, because GitHub renders it rather than the
  generator; `docs.yml` fails when it disagrees with `Cargo.toml`.

## [0.1.3] - 2026-08-25

### Fixed

- Straitjacket passes its own `no-comments` rule again. `site/wrangler.toml`
  explained itself in three lines the rule does not allow -- one past the ten
  the file header gets, and two more down beside the setting they described.
  The reasoning is intact, folded into the header.
- The `action` CI job fails on findings. It did not, so the first thing to
  notice the above was the release smoke test, after the tag was pushed and
  after the release was published. The check that catches a finding has to run
  where a finding can still be fixed.

## [0.1.2] - 2026-08-25

### Fixed

- `install.sh` reported a refused request as a repository with no releases. The
  fetch was piped straight into `sed`, so its exit status was hidden behind
  `head` and an empty parse looked like an empty repository -- the message sent
  whoever read it looking in the wrong place. It now reads the response before
  parsing it, and says what is usually true: GitHub allows 60 unauthenticated
  API requests an hour per address, CI runners share addresses, and the fix is
  a token or a pinned version.
- Downloads retry. A reset connection on the way to a release is not a reason
  to fail a build.
- A failed installation says so. In a checks list an install that could not
  happen and a scan that found something are both a red job ending in `exit
  code 1`; the action now annotates the first as an installation failure, and
  points at the `token` input when rate limiting is the cause.
- `action.yml` loads again. The `token` input's description explained itself
  with a live `${{ github.token }}`, and GitHub evaluates every expression it
  finds in a manifest, descriptions included -- so the sentence documenting the
  token stopped the action loading for everyone on `@main`. CI now uses the
  action on every change, because nothing else here reads that file.

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

[Unreleased]: https://github.com/PowderworksCode/straitjacket/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/PowderworksCode/straitjacket/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/PowderworksCode/straitjacket/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/PowderworksCode/straitjacket/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/PowderworksCode/straitjacket/releases/tag/v0.1.0
