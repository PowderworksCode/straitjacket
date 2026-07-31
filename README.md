# straitjacket

Straitjacket is an opinionated, deterministic source scanner for CI. It finds a
small set of code smells, applies explicit suppressions, and reports the results
consistently as human-readable text, JSON, or SARIF.

Straitjacket owns policy and reporting. Entl supplies codebase and parser
observations; Infact turns verified inputs into facts. No prose scoring or
language frontend lives in Straitjacket.

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
| `no-comments` | Opt-in mode that permits a 10-line file header and documentation comments, then flags ordinary comments. |
| `stray-todo` | Flags TODO, TBD, FIXME, and WIP markers left in comments. |
| `unused-marker` | Flags suppression markers that did not suppress anything. |
| `exact-clone` | Opt-in syntax-token clone detection. |
| `near-clone` | Opt-in clone detection with configured identifier and literal normalization. |
| `library-opportunity` | Flags local behavior equivalent to an API in an aspirational library. |
| `effect-capability` | Enforces direct providers and permitted transitive access for configured effects. |
| `analysis-incomplete` | Flags files that an enabled fact-backed analysis could not inspect. |

## Usage

```sh
cargo run -- .
cargo run -- src tests --only emoji,color
cargo run -- . --skip motion --max-lines 800
cargo run -- . --format json
cargo run -- . --sarif straitjacket.sarif
cargo run -- . --no-comments
cargo run -- instructions
cargo run -- facts sync
cargo run -- facts sync --offline
cargo run -- facts status
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

[facts]
parser-paths = ["tools/parsers"]
registries = ["ghcr.io/zmaril/infact-facts"]
dependencies = "automatic"
build-missing = false
exact-clones = true
near-clones = true
clone-exclude = ["tests/fixtures/"]

[[facts.builders]]
ecosystem = "cargo"
command = ["my-infact-builder", "build"]

[facts.exact]
min-tokens = 40
min-lines = 4

[facts.near]
min-tokens = 40
min-lines = 4
normalize-identifiers = true
normalize-literals = true
max-changed-percent = 15

[effects]
unlisted = "deny"
incomplete = "error"

[[effects.capabilities]]
name = "filesystem"
includes = ["file-read", "file-write"]
provided-by = ["src/adapters/filesystem/**"]
available-to = ["src/application/**", "src/bin/**"]

[aspirations]
libraries = [
  "cargo:itertools@0.15",
  "cargo:strum@0.28",
]
```

CLI values override the file, which overrides built-in defaults. Unknown
configuration keys and rule IDs are errors.

Effect capabilities use repository-relative glob patterns. `provided-by` is
where the underlying effectful API may be called directly. `available-to` is
where local callers may reach that provider transitively; provider paths are
implicitly available to themselves. `unlisted = "deny"` requires every known
effect to be assigned to one capability. `incomplete` accepts `"error"`,
`"warn"`, or `"ignore"` and controls effect-analysis diagnostics. An enabled
effect policy requires parser packs and at least one locked `call-effects` fact
pack. The initial repository analyzer resolves Rust call syntax and propagates
known external effects through local calls with DBSP.

`facts sync` reads exact dependency versions from repository lockfiles through
Entl, observes the active Rust compiler for the matching `rust-core` pack,
checks configured OCI registries for matching prebuilt packs, verifies their
contents, and writes `straitjacket.lock.toml`. Missing automatic packs are
reported and skipped; missing aspiration packs are errors. `facts sync
--offline` verifies only the TOML lock and local content-addressed cache.
Ordinary scans never run a compiler, resolve tags, contact a registry, or
invoke a builder.

GHCR is a cache, not the source of truth. A locally generated or private pack
uses the same manifest, cache, and lock format. Straitjacket reuses an already
locked local pack when its subject matches the configured aspiration, compiler,
or exact dependency version.

With `build-missing = true`, a configured ecosystem builder is invoked only
after prebuilt resolution fails. Straitjacket appends:

```text
--ecosystem <name> --package <name> --version <version>
--repository <path> --output <path>
```

The builder must place one OCI image layout at `--output`. Straitjacket verifies
the complete layout, checks its subject, imports it into the content-addressed
cache, and locks the resulting digest. `--prebuilt-only` prevents builder
execution.

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
