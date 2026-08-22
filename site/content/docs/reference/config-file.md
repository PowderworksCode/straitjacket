---
title: Config file
description: Check a straitjacket.toml into your repo to set defaults for every run.
order: 3
---

Instead of passing flags every time, check a **`straitjacket.toml`** into your
repo. Straitjacket picks it up automatically — from the current directory or any
parent — so every run (local or CI) starts from the same settings.

```toml
# straitjacket.toml
paths = ["src", "tests"]
skip = ["motion"]
max-lines = 800
theme-files = ["src/theme/tokens.css"]
file-size-exclude = ["notes/"]
no-comments = false
```

Every key is optional. Written out in full, the defaults are:

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

## Keys

Every key mirrors a [CLI flag](/docs/reference/cli) one-for-one, in
**kebab-case**:

| key | type | flag |
|-----|------|------|
| `paths` | list of strings | positional paths |
| `format` | `text` \| `json` \| `sarif` | `--format` |
| `only` | list of rule ids | `--only` |
| `skip` | list of rule ids | `--skip` |
| `max-lines` | number (0 disables `file-size`) | `--max-lines` |
| `max-nesting` | number (0 disables `deep-nesting`) | `--max-nesting` |
| `file-size-exclude` | list of path prefixes | — |
| `todo-exclude` | list of path prefixes | — |
| `theme-files` | list of paths allowed to define colors | — |
| `no-comments` | boolean | `--no-comments` ([no-comments mode](/docs/reference/rules#no-comments-mode)) |
| `include-json` | boolean | `--include-json` |
| `no-ignore` | boolean | `--no-ignore` |
| `no-fail` | boolean | `--no-fail` |
| `fail-on-unused-markers` | boolean | `--no-fail-on-unused-markers` |

An **unknown key is an error**, and so is an unknown rule id in `only`/`skip` —
a typo'd setting is surfaced, not silently ignored. The rule ids are the ones in
the [rules reference](/docs/reference/rules).

`theme-files` designates the files that are *allowed* to define color literals,
so a palette or token module doesn't have to be papered over with markers.
`file-size-exclude` and `todo-exclude` are path prefixes those two rules skip.

## Precedence

Settings layer in this order, each overriding the one before:

1. Built-in defaults
2. `straitjacket.toml`
3. CLI flags

So a `--max-lines 0` on the command line wins over `max-lines = 800` in the file,
which wins over the default of 1500.

## Discovery

Straitjacket looks for `straitjacket.toml` in the current directory and walks up
to the filesystem root, using the first it finds. When a file is loaded it prints
a one-line note to stderr, so stdout stays clean for `--format json` and
`--format sarif`.

- `--config <path>` — use a specific file instead of discovering one.
- `--no-config` — ignore any checked-in configuration and use only flags and
  defaults.

## Sections from removed rules

Configuration written for an older Straitjacket that carried the fact-backed
rules — `[facts]`, `[effects]`, `[errors]` — is **rejected with an error naming
the rules that went away**, rather than being quietly ignored. Delete the
section; those rules are not coming back in this line of the tool.

## With the GitHub Action

The [Action](/docs/reference/github-action) runs Straitjacket inside your
checked-out repo, so a committed `straitjacket.toml` is picked up with no extra
configuration. Leave the Action's inputs blank to defer to the file; set an input
to override it for that workflow.
