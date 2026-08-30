# Changelog

All notable changes to Straitjacket are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `stray-const`, an opt-in rule that reports `SCREAMING_SNAKE_CASE` constants
  declared outside the files named by `const-files`. A constant is a decision
  the program has made -- a limit, a path, a key, a magic number somebody
  named -- and scattered across the tree those decisions cannot be read as a
  set, so the same one gets made twice under two names. `const-files` is
  `theme-files` for constants: a declaration inside one is what the rule asks
  for, everywhere else is an error. Enable with `stray-const = true` or
  `--stray-const`; enabling it without naming a file is refused, because every
  constant would be a finding with nowhere to move it.
- The rule needs no grammar, so unlike `test-quality` it covers
  all eighteen languages straitjacket calls structured code and never reaches
  the network. It reports declarations rather than uses -- referencing a
  constant is the point of having one -- and it reads code rather than text,
  so a declaration commented out or quoted inside a string is not one. A bare
  `NAME = value` counts as a declaration only in the languages that spell it
  that way, and in Python, Ruby and Shell only at the left margin, which is
  what keeps every `enum` member from being a finding.

### Changed

- `rules::comments` now yields the file's comments and a code-only view from
  one traversal (`comments::code`), rather than only the comments. Masking
  preserves byte offsets, so a column found in the masked line is the column
  in the real one. One lexer answers where the code stops and the prose
  starts; answering it in two places is how the two answers come to disagree.

## [0.2.0] - 2026-08-30

### Added

- `test-quality`, opt-in, which flags tests that weaken what they prove --
  today a loop or a conditional in a test body. It parses the file with a
  [treebank](https://github.com/PowderworksCode/treebank) grammar rather than
  matching text, and reads a test the way the language writes one: `#[test]`,
  `@Test`, `it(...)`, `TEST(...)`, `test "..."`. Ten languages: python, ruby,
  rust, java, typescript, javascript, c, c++, shell, zig.

  The analysis is [beamte](https://github.com/PowderworksCode/beamte)'s, called
  once per file; this crate does what beamte refuses to -- fetching a grammar,
  parsing, severity, suppression, reporting. A rule added to beamte appears
  here with no change to this one.

  Opt-in because the first run downloads a grammar. Packs are fetched per
  language and only once a file already looked like a test, so a Python
  repository never downloads the Java grammar. A grammar that will not load
  reports the file as **not read** rather than as clean: a scanner that goes
  quiet when its parser breaks is how a broken parser becomes a green build.

  Files are not chosen by path alone. Rust and Zig keep tests inside ordinary
  source files, so a path rule would make this silent for both -- the failure
  it exists to prevent.

- `--test-quality`, and `test-quality` in `straitjacket.toml`, which turn the
  rule on beside the nine default ones. Without it the only way to reach an
  opt-in rule was `only`, which runs it and nothing else, leaving a scan with
  the default rules or this one and never both. `no-comments` had a switch
  and this did not.

- `test-rules` in `straitjacket.toml`, naming which of beamte's rules run.
  Unset runs all of them. An unknown name is refused when settings are read,
  listing what beamte does have, because a typo that silently disables a rule
  is the same quiet failure as a test that passes by being empty.


- [Update Straitjacket](https://straitjacket.dev/guides/updating), a guide for
  the thing every installed tool eventually needs and this one never documented:
  re-run the installer, roll back with `STRAITJACKET_VERSION`, bump the Action
  tag, and keep a machine on the release CI runs. Nothing checks for a new
  version behind your back, and the guide says so -- for a gate, an update that
  arrives on its own is a red build nobody caused.

### Changed

- `beamte` and `treebank` are ordinary version dependencies. Both were git
  references while the work that needed them was unreleased, and a versionless
  dependency is what `cargo publish` refuses, which is why this crate could not
  be published at all. beamte 0.1.0 and treebank 0.3.0 fix that, and the lockfile
  now carries no `git+` source.


- The Action's tag pins the scanner. `version` now defaults to the release the
  Action ref ships, read from the `Cargo.toml` beside `action.yml` in that ref,
  rather than to `latest`, meaning `@v0.1.3` runs v0.1.3 and a release published
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

- `scripts/publish.sh --dry-run` verifies something. Its skip for a version
  already on the registry ran before the dry run as well as before the upload,
  so on every commit that did not bump the version the release gate reported
  success having packaged nothing and compiled nothing. The skip now applies
  only to `--execute`. `--help` had the matching defect, a hardcoded line range
  that truncated once the header changed, and now reads the comment block.

- The site and README say how many rules there are. Both said "nine of the ten"
  and named `no-comments` as "the tenth", which stopped being true when
  `test-quality` landed. `straitjacket --list-rules` is the authority: eleven
  rules, nine on by default, `no-comments` and `test-quality` opt-in.


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
  finds in a manifest, descriptions included, and the sentence documenting the
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
