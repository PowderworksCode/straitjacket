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

See [Suppression markers](/docs/reference/suppression-markers) for the full syntax.

### `emoji` and test fixtures

`emoji` is lexical and does not know a string literal from a comment, so a test
that deliberately exercises Unicode — `remove_extension("💖.txt")` — is flagged
like any other emoji in source. That is the rule working as specified, not a
misfire, but it is the most common first-run surprise on a repository that
handles text. A file-scoped marker is the right answer for a fixture module:

```rust
// straitjacket-allow-file:emoji — these fixtures test Unicode handling
```

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

or check it into [`straitjacket.toml`](/docs/reference/config-file) with
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
[marker](/docs/reference/suppression-markers) as usual, or grandfather a file
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
