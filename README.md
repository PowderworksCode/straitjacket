# straitjacket

Straitjacket is an opinionated, deterministic source scanner for CI. It finds a
small set of code smells, applies explicit suppressions, and reports the results
as human-readable text, JSON, or SARIF.

It has no service to call, no cache to warm, and no dependency outside
crates.io. Point it at a directory and it prints findings.

```sh
cargo install --git https://github.com/PowderworksCode/straitjacket
straitjacket .
```

```text
src/theme/button.css:12:14  [color]  #ff6600
  hardcoded color literal
  help: use a theme token or CSS variable
straitjacket: 1 error(s), 0 warning(s) across 84 file(s); 0 suppressed
```

The process exits `0` when clean, `1` for error-level findings, and `2` for a
configuration or operational failure. `--no-fail` reports findings and exits
successfully, which is the shape for a first run against an existing repository.

## Rules

| rule | default | behavior |
| --- | --- | --- |
| `color` | on | Flags hardcoded CSS color literals. |
| `deep-nesting` | on | Flags code nested beyond eight indentation levels. |
| `emoji` | on | Flags emoji glyphs in source and Markdown. |
| `file-size` | on | Flags files over 1,500 lines. |
| `inline-font` | on | Flags literal font-family stacks, allowing tokens and CSS variables. |
| `inline-svg` | on | Flags inline SVG in component source. |
| `motion` | on | Flags ad-hoc transitions, animations, and keyframes. |
| `stray-todo` | on | Flags TODO, TBD, FIXME, and WIP markers left in comments. |
| `unused-marker` | on | Flags suppression markers that did not suppress anything. |
| `no-comments` | opt-in | Permits a 10-line file header and documentation comments, then flags ordinary comments. |

Every rule is lexical. Straitjacket reads files and applies patterns; it does
not parse, resolve types, or follow calls. That is the reason it runs on any
repository without setup, and it is also the limit of what it can tell you.

`straitjacket --list-rules` prints this table from the binary, which is the
authority if the two disagree.

## Usage

```sh
straitjacket .
straitjacket src tests --only emoji,color
straitjacket . --skip motion --max-lines 800
straitjacket . --format json
straitjacket . --sarif straitjacket.sarif
straitjacket . --no-comments
straitjacket instructions
```

The SARIF output validates against the SARIF 2.1.0 schema and is what GitHub
code scanning ingests.

`straitjacket instructions` prints a short, agent-facing description of the
policy resolved from the repository's `straitjacket.toml`, suitable for a
`CLAUDE.md`, an `AGENTS.md`, or an agent hook.

## Suppression

A rule-scoped line marker suppresses a finding on the same line:

```ts
const brand = "#ff6600"; // straitjacket-allow:color — fixed brand color
```

A file marker suppresses findings anywhere in that file:

```css
/* straitjacket-allow-file:color — this file defines the palette */
```

The bare forms suppress all applicable rules. Markers that suppress nothing are
reported by `unused-marker`; disable that check with
`--no-fail-on-unused-markers`.

## Configuration

Straitjacket discovers `straitjacket.toml` in the current directory or a
parent. Every key is optional, and the values below are the defaults:

```toml
paths = ["."]
only = []
skip = []
max-lines = 1500
file-size-exclude = []
todo-exclude = []
theme-files = []
max-nesting = 8
no-comments = false
include-json = false
no-ignore = false
no-fail = false
fail-on-unused-markers = true
```

CLI values override the file, which overrides built-in defaults. Unknown
configuration keys and unknown rule IDs are errors.

`theme-files` designates the files that are allowed to define color literals,
and `file-size-exclude` and `todo-exclude` are path prefixes those two rules
skip.

## What it scans

Straitjacket walks the given paths, honoring `.gitignore`, `.ignore`,
`.git/info/exclude`, and the hidden-file convention. `--no-ignore` turns all of
that off. `.git` is never scanned.

A file's language comes from its extension, then its whole filename, then its
`#!` line. Files in a language Straitjacket does not know are skipped, which is
how binaries and generated assets fall out without a list. JSON is known but
skipped unless `include-json` is set, because a JSON file is data far more often
than it is source.

Rules then narrow further. `color` and `motion` run wherever style values can
appear, which includes Vue, Svelte, and JSX as well as CSS; `deep-nesting` runs
only on languages that nest executable code; `emoji` runs everywhere except data
formats.

## Upgrading from an earlier build

Straitjacket used to carry eight more rules — `exact-clone`, `near-clone`,
`library-opportunity`, `effect-capability`, `effect-barrier`, `unknown-barrier`,
`error-discard`, and `analysis-incomplete`. Each of them read facts from a
package that was never published, so none of them could run outside the
repository that built them. They are gone, along with the `facts sync` and
`facts status` subcommands and the `[facts]`, `[effects]`, and `[errors]`
configuration sections.

A configuration that still names one of them fails at startup with a message
naming the rule. Delete the section or the rule ID and the rest of the file
keeps working.

## Architecture

Rules emit unsuppressed candidates. The scanner applies suppression once, then
reporters render the surviving findings. Findings carry related locations and
ordered evidence paths.

Rule identifiers are static `RuleKey` values owned by inventory registrations.
Configuration strings resolve against that catalog before scanning; findings,
suppression, reporting, and generated instructions then share the resolved key.
Every rule lives under `src/rules` with its generated instruction text and
submits itself through `inventory`; scanner construction sorts registrations and
rejects invalid names, duplicate keys, and mismatched factories.

`src/language.rs` holds the language table — extensions, filenames, shebangs,
comment syntax, and the facets rules select on. `src/walk.rs` is the file walk.
Adding a language is an entry in that table.

## License

MIT.
