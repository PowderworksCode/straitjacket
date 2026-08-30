---
title: Rules
description: Every built-in rule, what it flags, the file types it runs on, and how to tune or disable it.
order: 1
---

The built-in rules are intentionally **generic** — no framework or
single-language assumptions — so the same binary works across all your repos.
Everything is on by default; you ratchet down with `--skip`. (The one exception
is [`no-comments`](#no-comments-mode), an opt-in mode.) Each rule only looks
at the file types where it makes sense (e.g. `color` ignores `.json`,
`inline-svg` only scans component sources).

Every rule is **lexical**. Straitjacket reads files and applies patterns; it
does not parse, resolve types, or follow calls. That is why it runs on any
repository without setup, and it is also the limit of what it can tell you.

Run `straitjacket --list-rules` to see them with their one-line descriptions.
That output comes from the binary, so it is the authority if this page and your
installed version ever disagree.

## The rules

| rule | default | flags |
|------|---------|-------|
| `emoji` | on | emoji glyphs in code, comments, strings, and Markdown (a reliable LLM tell). Color emoji, VS16-presented glyphs, and flag sequences — but **not** text symbols like `©` `™` `✓`, arrows, dashes, or the geometric star. |
| `color` | on | hardcoded color literals — hex (`#1e1e1e`) and CSS color functions (`rgb()`, `rgba()`, `hsl()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`, `color()`). Use a theme token / CSS variable instead. Files listed in `theme-files` are allowed to define them. |
| `inline-svg` | on | hand-rolled inline `<svg>` in component code — extract it into a named, reusable icon. |
| `inline-font` | on | inline `font-family` **literal stacks** — a quoted font or a multi-family list (`Inter, sans-serif`). A token reference (`fontFamily: MONO`), a CSS variable, or a bare generic (`monospace`) is fine. Define the font once and reference a token. |
| `motion` | on | ad-hoc `transition` / `animation` / `@keyframes` — centralize motion so it can be tuned or disabled. |
| `file-size` | on | files longer than the line budget (default **1500**) — sprawling single files are a common LLM tell. Tune with `--max-lines`, disable with `--max-lines 0` or `--skip file-size`. Exempt path prefixes with `file-size-exclude`. |
| `deep-nesting` | on | lines indented past the nesting budget (default **8**) — deeply nested logic is hard to follow. Depth is read off leading indentation (which is canonical when a formatter is enforced), so it's language-agnostic and needs no parser. Runs on programming-language sources only (not markup/data). Tune with `--max-nesting`, disable with `--max-nesting 0` or `--skip deep-nesting`, or exempt code that legitimately nests deeper with an allow marker. |
| `stray-todo` | on | deferred-work markers left in comments — `TODO`, `TBD`, `FIXME`, `WIP`. Do the work now, or record it in an issue the repository tracks. Exempt path prefixes with `todo-exclude`. |
| `unused-marker` | on | a suppression marker that did not suppress anything — the finding it was written for is gone, so the marker is stale. Turn it off with `--no-fail-on-unused-markers`. |
| `no-comments` | **opt-in** | every comment, in every language it knows (`//`, `/* */`, `#`, `--`, `<!-- -->`). See [no-comments mode](#no-comments-mode) below. |
| `test-quality` | **opt-in** | tests that weaken what they prove — currently a loop or a conditional in a test body. Parses the file with a treebank grammar, so it reads a test the way the language writes one: `#[test]`, `@Test`, `it(...)`, `TEST(...)`, `test "..."`. See [test quality](#test-quality) below. |
| `env-vars` | **opt-in** | code that reads the process environment where nothing declares it — `std::env::var`, `os.environ`, `process.env`, `ENV[...]`, `System.getenv`, `getenv`. Files listed in `env-files` are the declared configuration edge and are allowed to read it. See [environment variables](#environment-variables) below. |

### `deep-nesting` and embedded DSLs

`deep-nesting` reads depth from leading indentation and deliberately does **not**
tokenize the language — that's what keeps it a single, language-agnostic pass. The
trade-off: a multi-line **string literal** that embeds an indented DSL — YAML or
JSON in a Python `"""…"""`, an HTML/SQL heredoc, a template literal — is counted as
if that indentation were code nesting, so a deep enough block can trip the rule even
though the surrounding code is flat.

This is by design, not a bug to detect around: reliably knowing "am I inside a string
literal?" needs a per-language parser, which the rule exists to avoid. Suppress the
false positive with a marker instead.

For a file that carries a lot of embedded DSL — test fixtures, template modules — a
**file-scoped** marker is cleanest, because it's a real comment outside any string
and doesn't touch the literal's contents:

```python
# straitjacket-allow-file:deep-nesting — this module is embedded YAML fixtures
```

A **line-scoped** marker also works — but it has to sit on the line the finding
points at, which for embedded content is *inside* the string literal, so it becomes
part of that text (usually fine as a DSL comment, but check it doesn't change what
the fixture means):

```python
CI = """\
jobs:
  build:
    steps:
      - run: deep | embedded | yaml  # straitjacket-allow:deep-nesting
"""
```

See [Suppression markers](/reference/suppression-markers) for the full syntax.

### `emoji` and test fixtures

`emoji` is lexical and does not know a string literal from a comment, so a test
that deliberately exercises Unicode — `remove_extension("💖.txt")` — is flagged
like any other emoji in source. That is the rule working as specified, not a
misfire, but it is the most common first-run surprise on a repository that
handles text. A file-scoped marker is the right answer for a fixture module:

```rust
// straitjacket-allow-file:emoji — these fixtures test Unicode handling
```

## test quality

Rules about what a test proves, rather than how the code is written. They come
from [beamte](https://github.com/PowderworksCode/beamte), which implements them
against the [treebank](https://treebank.dev) node vocabulary; straitjacket runs
them and reports what comes back.

Today that is one rule, `test-logic`: a loop or a conditional in a test body,
restating [Testing on the Toilet: Don't Put Logic in
Tests](https://testing.googleblog.com/2014/07/testing-on-toilet-dont-put-logic-in.html).
A test is a concrete input/output pair, so state the values directly rather
than computing them, and split the cases into separate tests.

```sh
straitjacket --only test-quality
```

or in [`straitjacket.toml`](/reference/config-file):

```toml
only = ["test-quality"]
test-rules = ["test-logic"]   # optional: unset runs every test rule beamte has
```

**It is opt-in because it reaches the network.** The grammar for a language is
downloaded the first time a test file in that language is scanned, verified
against a sha256 in treebank's manifest, and cached content-addressed after
that. Grammars are fetched per language and only once a file has already looked
like a test, so a Python repository never downloads the Java grammar. If a
grammar cannot be fetched, the file is reported as **not read** rather than
passing quietly — a checker that goes silent when its parser is missing reads
exactly like a clean suite.

### Languages

Ten, being the ones treebank publishes a grammar for: Python, Ruby, Rust, Java,
TypeScript, JavaScript, C, C++, Shell and Zig. A language with no grammar is
not scanned by this rule at all.

Test detection is per language, because no two mark a test the same way:

| shape | looks like | languages |
|---|---|---|
| a name | `def test_adds`, `void test_adds()` | Python, Ruby, Shell, C |
| an attribute | `#[test]`, `@Test` | Rust, Java |
| an invocation taking a body | `it("adds", ...)`, `TEST(Suite, Adds)` | TypeScript, JavaScript, Ruby, C, C++ |
| a declaration of its own | `test "adds" { ... }` | Zig |

Rust and Zig keep tests inside ordinary source files, so files are not
prefiltered by path alone.

A suite is not a test: a loop in `describe(...)` is generating cases, not
computing an expectation, and is left alone. In Ruby and JavaScript, iterating
with a block — `users.each do |u|`, `users.forEach(...)` — counts as a loop,
since that is the form those languages actually use.


## no-comments mode

Where — you guessed it — no comments are allowed. The `no-comments` rule flags
**every comment**: line, block, doc comments, pragmas, all of them. The position
is the maximalist one: a comment is a place where the code stopped speaking for
itself, and LLMs narrate relentlessly (`// increment the counter`). If it
matters, say it in the code; if it's history, say it in the commit message.

It's the one rule that is **off by default** — the rest of the rule set runs at
its max and you ratchet down, but comments are ordinary in most codebases, so
this one is a mode you opt into:

```sh
straitjacket --no-comments        # the mode, alongside every other rule
straitjacket --only no-comments   # just show me the comments (implies the mode)
```

or check it into [`straitjacket.toml`](/reference/config-file) with
`no-comments = true`.

A leading file header is permitted — ordinary comments are allowed in the first
10 lines, before code begins — and documentation comments are allowed wherever
they feed language documentation tooling, including rustdoc and JSDoc.

It knows the comment syntax of the common extensions — C-family (`//`, `/* */`),
hash languages (`#`: Python, Ruby, shell, YAML, TOML), SQL (`--`), CSS
(`/* */`), and HTML/Vue/Svelte (`<!-- -->`) — and tracks string literals, so a
`//` in a URL or a `#` inside a quoted value isn't a comment. A block comment
reports once, at its opening delimiter.

Two things that look like comments are never flagged:

- **Shebang lines** (`#!/usr/bin/env bash`) — interpreter directives, not
  commentary.
- **Suppression markers** — a comment carrying `straitjacket-allow[-file]` is
  the escape hatch at work. That's also what keeps the escape hatch usable for
  every other rule while the mode is on; a marker that suppresses nothing is
  still caught by `unused-marker`.

The scanner is deterministic, not a per-language parser, so an exotic literal (a
regex literal, a heredoc) can fool it — suppress with a
[marker](/reference/suppression-markers) as usual, or grandfather a file
with `straitjacket-allow-file:no-comments`.

## Severity and exit code

The report distinguishes **error** and **warning** findings, and warnings are
tagged `(warn)` in the output. Every built-in rule shipped today reports at
error level, so a normal run's summary always reads `0 warning(s)`; the
distinction exists for rules that report advisory findings.

The process exits **1** when there's any error-level finding — so CI fails — and
**0** when the scan is clean or found only warnings. It exits **2** on a
configuration or operational failure, which is a different thing from a finding
and should not be mistaken for one. Override with `--no-fail` to report
findings and still exit 0, which is the shape for a first run against an
existing repository.

## Defaults at a glance

| setting | default | flag |
|---------|---------|------|
| file-size line budget | 1500 | `--max-lines` |
| deep-nesting depth budget | 8 | `--max-nesting` |
| no-comments mode | off | `--no-comments` |
| scan `.json` | off | `--include-json` |
| respect `.gitignore` | on | `--no-ignore` |
| fail on unused markers | on | `--no-fail-on-unused-markers` |
| fail on findings | on | `--no-fail` |

## environment variables

An environment variable read in the middle of ordinary code is configuration
no signature admits to: the function behaves differently on two machines and
nothing in its declaration says why. `env-vars` reports every such read —
`std::env::var` in Rust, `os.environ` and `os.getenv` in Python, `process.env`
in TypeScript and JavaScript, `ENV[...]` in Ruby, `System.getenv` and
`System.getProperty` in Java, `getenv` in C and C++, `std.process.getEnvVarOwned`
in Zig. The finding is [beamte](https://github.com/PowderworksCode/beamte)'s
`env-read`, restating [Test
Sizes](https://testing.googleblog.com/2010/12/test-sizes.html): a small test
may not touch system properties, and a component that reads the environment
mid-body forces that violation on every small test that executes it.

```sh
straitjacket --env-vars
```

or in [`straitjacket.toml`](/reference/config-file):

```toml
env-vars = true
env-files = ["src/config.rs"]   # the declared configuration edge
```

`env-files` names the files that *are* the configuration edge — the one module
that reads the environment and hands values on as arguments. Reads there are
licensed; reads anywhere else are errors. It is `theme-files` for the
environment: designate the edge instead of papering readers over with markers.
The exception with a story — a genuinely per-invocation override — takes a
[suppression marker](/reference/suppression-markers), which must carry one.

**It is opt-in because it reaches the network**, exactly as `test-quality` is:
the grammar for a language is downloaded the first time a file in that language
carries an environment-shaped token, verified and cached content-addressed
after that. A file whose grammar cannot be fetched is reported as **not read**
rather than passing quietly.

Files are prefiltered by cheap substrings (`env::var`, `environ`,
`process.env`), so a file that cannot contain a read is never parsed and the
rule stays affordable over a whole repository.

Nine languages: Python, Ruby, Rust, Java, TypeScript, JavaScript, C, C++ and
Zig. Shell is deliberately not among them — `$VAR` is the language's own
variable model, and flagging every expansion would be flagging the language.
Compile-time reads are not findings either: Rust's `env!` resolves when the
build runs, against variables the build declares, which is the announced
channel this rule steers reads toward.
