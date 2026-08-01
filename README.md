# straitjacket

> **Early and unstable.** Straitjacket is under active development, has not been
> released, and is not ready for use by anyone outside this repository.
> Configuration keys, rule identifiers, and output shapes change without notice
> or migration. Several rules depend on Infact packs that do not exist yet.

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
| `library-opportunity` | Flags local behavior equivalent to an API in a library the repository already depends on. |
| `effect-capability` | Enforces direct providers and permitted transitive access for configured effects. |
| `effect-barrier` | Flags an effect reached from a callable whose source marker forbids it, however deep the call. |
| `unknown-barrier` | Flags a barrier marker that names no configured barrier, and so forbids nothing. |
| `error-discard` | Flags a fallible expression whose error is dropped instead of returned to the caller. |
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

[[effects.barriers]]
name = "hot-loop"
denies = ["allocate", "block", "file-read", "file-write", "network"]

[errors]
deny = ["let-underscore", "ok-discard", "err-arm", "ok-binding", "iterator-drop"]
ambiguous = "skip"
tests = "ignore"
allowed-in = ["src/bin/report.rs"]
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

An effect barrier asks the other question. A capability says where an effect may
live, which is about paths; a barrier says what one callable may reach, which is
about the call graph, and no arrangement of files answers it. A hot loop may call
anything in the repository and still must not allocate.

A barrier is declared in source, because the callable is what carries it: a path
pattern goes stale when the function moves and a comment on the function does
not. `barrier` is a directive alongside `allow`, so a barrier named `hot-loop`
is written on a callable as `straitjacket-barrier:hot-loop`:

```rust
// straitjacket-barrier:hot-loop
pub fn tick(&mut self) -> Frame {
    self.render()
}
```

Every denied effect that callable reaches — directly, or through any call below
it, however deep — is a finding, reported at the operation rather than at the
barrier, with the chain of calls that carried it:

```text
src/support.rs:2:1  [effect-barrier]  hot-loop
  allocate effect is denied below the hot-loop barrier on hot::tick
  help: hot::tick reaches this through 2 calls; move the allocate out of the
        path or hoist it above the barrier
  via: src/hot.rs:3:1: hot::tick calls support::render
  via: src/support.rs:2:1: support::render calls rust:allocation:format!
```

A marker that names no configured barrier forbids nothing and would otherwise
say nothing about it, so `unknown-barrier` reports it. That covers a misspelled
name and a barrier deleted from configuration whose markers were left behind;
both read as a live guarantee and are decoration. It is the barrier counterpart
to `unused-marker`, and it is on by default whether or not any barrier is
configured.

Barriers inherit the precision of the effect analysis under them, which
under-approximates: a call syntax cannot resolve contributes no effect, so a
clean barrier is a weaker claim than it appears. Set `incomplete = "error"` and
prefer resolved observations when a barrier is load-bearing. Allocation origins
are the standard-library operations that reach the allocator; `Vec::new` and an
empty `vec![]` are not among them, and `clone` and `to_owned` cannot be judged
without the receiver's type and so are not reported at all.

`[errors]` turns on `error-discard`, which reports a fallible expression whose
error is dropped rather than returned. Infact supplies the sites and the shape
of each one; this section decides which shapes a repository refuses to carry.
`deny` lists the forms that are findings, from `let-underscore`, `ok-discard`,
`unwrap-or`, `err-arm`, `ok-binding`, `iterator-drop`, `cause-erased`, and
`panic`.

A finding says how far the failure could have travelled, which the enclosing
signature alone cannot answer. Infact resolves the callers, so a discard is
reported as reaching one of four places: the callable itself returns `Result`
and declined to use it; a caller above returns `Result` and could have been
told; every caller up to a root is infallible, so nothing can report it; or the
callers could not be resolved from syntax, in which case the report says that
rather than guessing. The evidence lists the calls that would have carried it.

`.ok()` and an `Err(_)` arm name `Result` and nothing else. `.unwrap_or_default()`
reads the same on `Option`, and the analyzer resolves no types, so those sites
are reported only when `ambiguous` is `"warn"` or `"error"`; the default,
`"skip"`, leaves them alone rather than guessing. `tests` accepts `"ignore"`,
`"warn"`, or `"error"` for sites inside `#[cfg(test)]` or `#[test]`.
`allowed-in` exempts paths that are permitted to discard, such as a top-level
reporter that is already the last handler. The rule needs parser packs and no
fact pack.

Declared dependencies are the configuration for `library-opportunity`. There is
no list of libraries to maintain: adding a dependency is the request to use it
well. `facts sync` locks an Infact pack for every dependency Infact can describe,
and the rule then reports local code equivalent to an API those libraries
already provide. Dependencies without a published pack are skipped, and
`dependencies = "none"` turns the whole mechanism off.

`facts sync` reads exact dependency versions from repository lockfiles through
Entl, checks configured OCI registries for matching prebuilt packs, verifies
their contents, and writes `straitjacket.lock.toml`. Missing packs are reported
and skipped. An enabled effect policy additionally observes the active Rust
compiler and requires the matching `rust-core` pack. `facts sync --offline`
verifies only the TOML lock and local content-addressed cache. Ordinary scans
never run a compiler, resolve tags, contact a registry, or invoke a builder.

GHCR is a cache, not the source of truth. A locally generated or private pack
uses the same manifest, cache, and lock format. Straitjacket reuses an already
locked local pack when its subject matches the active compiler or an exact
dependency version.

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
