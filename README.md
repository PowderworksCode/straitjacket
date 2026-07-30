# straitjacket

Straitjacket is an opinionated, deterministic source scanner for CI. It finds a
small set of code smells, applies explicit suppressions, and reports the results
consistently as human-readable text, JSON, or SARIF.

This recreation starts with source-level checks and reporting infrastructure.
It deliberately contains no prose scoring and no semantic-analysis frontend.

## Rules

| rule | behavior |
| --- | --- |
| `emoji` | Flags emoji glyphs in source and Markdown. |
| `color` | Flags hardcoded CSS color literals. |
| `inline-svg` | Flags inline SVG in component source. |
| `inline-font` | Flags literal font-family stacks while allowing tokens and CSS variables. |
| `motion` | Flags ad-hoc transitions, animations, and keyframes. |
| `file-size` | Flags files over 1,500 lines by default. |
| `deep-nesting` | Flags code nested beyond eight indentation levels by default. |
| `no-comments` | Opt-in mode that flags comments while respecting strings and shebangs. |
| `stray-todo` | Flags TODO, TBD, FIXME, and WIP markers left in comments. |
| `unused-marker` | Flags suppression markers that did not suppress anything. |

## Usage

```sh
cargo run -- .
cargo run -- src tests --only emoji,color
cargo run -- . --skip motion --max-lines 800
cargo run -- . --format json
cargo run -- . --sarif straitjacket.sarif
cargo run -- . --no-comments
cargo run -- instructions
```

`straitjacket instructions` prints a short, agent-facing description of the
policy resolved from the repository's `straitjacket.toml`.

The process exits `0` when clean, `1` for error-level findings, and `2` for a
configuration or operational failure. `--no-fail` reports findings and exits
successfully.

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
parent:

```toml
paths = ["src", "tests"]
skip = ["motion"]
max-lines = 1000
file-size-exclude = ["notes/"]
todo-exclude = ["notes/"]
theme-files = ["src/theme.css"]
max-nesting = 5
no-comments = false
include-json = false
no-ignore = false
no-fail = false
fail-on-unused-markers = true
```

CLI values override the file, which overrides built-in defaults. Unknown
configuration keys and rule IDs are errors.

## Architecture

File rules emit unsuppressed candidates. The scanner applies suppression once,
then reporters render the surviving findings. Findings support related locations
and ordered evidence paths so future semantic-analysis producers can integrate
without becoming part of Straitjacket's detection machinery.

Rule identifiers are static `RuleKey` values owned by inventory registrations.
Configuration strings resolve against that catalog before scanning; findings,
suppression, reporting, and generated instructions then share the resolved key.
Every rule lives under `src/rules` with its generated instruction text and
submits itself through `inventory`; scanner construction sorts registrations
and rejects invalid names, duplicate keys, and mismatched factories.
Entl owns repository traversal, language detection, and shallow lexical facts
such as comment syntax. Straitjacket retains scan-path selection and each
rule's language policy.
