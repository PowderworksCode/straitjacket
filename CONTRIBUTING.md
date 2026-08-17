# Contributing to Straitjacket

Straitjacket encodes one person's taste in deterministic checks. That means the
most valuable thing you can send is not usually a patch — it is a **concrete
example** of something an LLM wrote that made you go "Yuck!", or a place
Straitjacket said "Yuck!" and was wrong.

## Reporting

[Open an issue](https://github.com/PowderworksCode/straitjacket/issues). The
three kinds most wanted:

- **A new smell.** A pattern that generalizes across repositories and is visible
  in the *text* of a file. Include the smallest snippet that shows it.
- **A false positive.** A place a rule fires on something genuinely fine.
  Include the smallest file that reproduces it, and say which rule.
- **Coverage.** A rule that should fire on a file type it currently skips, or
  skips one it shouldn't.

## What makes a good rule

Every rule is **lexical**: Straitjacket reads files and applies patterns. It does
not parse, resolve types, or follow a call from one file into another. That is
deliberate — it is why the tool runs on any repository with no setup, and it is
the boundary a proposal has to fit inside.

A rule is a good fit when it is:

- **Surface-visible.** Decidable from the bytes of one file.
- **General.** Not tied to one framework or one language.
- **Deterministic.** The same input always produces the same finding, at the
  same line and column.
- **Escapable.** A legitimate exception can be suppressed with a marker.

Rules that need real program understanding — does this value flow into that
component, is this clone semantically the same as that one — are out of scope.
Asking a lexical scanner to do that produces a tool that is confidently wrong in
ways nobody can audit.

## Working on the code

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
shellcheck -s sh install.sh
```

Straitjacket scans itself under its own rules, so run it before opening a PR:

```sh
cargo run --release -- .
scripts/rules-manifest.sh --check
```

It should print `ok`. Note that `straitjacket.toml` in this repository sets
`no-comments = true`: an ordinary comment is only allowed in a file's leading
10-line header, before code begins. If a comment explains a decision the code
cannot, hoist it to the header; otherwise make it structural.

### Adding a rule

Rules live in `src/rules/` and register themselves with `inventory::submit!`, so
a new rule is a new module plus a `mod` line — there is no central list to
update. Follow the shape of an existing single-purpose rule such as
`src/rules/stray_todo.rs`, and add tests alongside the ones in `tests/`.

A new rule needs:

1. A stable id, lowercase and hyphenated, that reads as the thing it flags.
2. A one-line description for `--list-rules`.
3. A `help:` line saying what to do about the finding, not just what is wrong.
4. Tests covering the positive case, a near miss that must *not* fire, and
   suppression by marker.
5. An entry in the [rules reference](site/content/docs/reference/rules.mdx) and
   the README table.
6. A regenerated rule manifest:

```sh
scripts/rules-manifest.sh
```

That exports the rule set from the binary into `site/content/rules.json`. CI
runs `scripts/rules-manifest.sh --check` and fails if the committed manifest has
drifted, and the site's tests check the documentation against it — a rule that
is not documented, a page that names a rule the binary does not have, or a
default quoted at a stale value all fail the build. That check exists because
the website once described six rules this repository never carried.

New rules default to on. If a rule is too noisy to be on by default, that is
usually a sign it is not general enough yet.

## The documentation site

[straitjacket.dev](https://straitjacket.dev) is built from `site/` — Next.js with
Fumadocs, statically exported. Content is MDX under `site/content/docs/`,
organized by [Diátaxis](https://diataxis.fr/).

```sh
cd site
bun install
bun run dev
```

Documentation and behavior are expected to change in the same pull request. The
site describing rules the binary does not have is the exact failure this project
has already had once.

## Releasing

Maintainers only. Releases are tag-driven: update `CHANGELOG.md` and the version
in `Cargo.toml`, tag, and let `release.yml` build the archives and publish the
crate. `scripts/publish.sh --dry-run` runs on every CI build, so the publish path
is exercised long before a tag depends on it.

## License

Contributions are accepted under the [MIT license](LICENSE). The banner image is
CC BY 4.0 and is not covered by it; see the README.
