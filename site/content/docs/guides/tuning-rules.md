---
title: Tune or disable rules
description: Run a subset of rules, adjust thresholds, and change how findings are reported.
order: 4
---

Straitjacket runs every rule at its strictest by default. You ratchet *down* from
there. This guide covers the common adjustments; the full flag list is in the
[CLI reference](/docs/reference/cli).

## Run only some rules

```sh
straitjacket --only emoji,color   # nothing but these two
straitjacket --skip motion        # everything except this rule
```

`--only` and `--skip` take comma-separated rule ids. An **unknown id is an
error** — the run stops with exit code `2` rather than silently scanning with a
rule you thought you had turned off. See every id in the
[rules reference](/docs/reference/rules) or with `straitjacket --list-rules`.

## Adjust thresholds

Two rules have a tunable number:

```sh
straitjacket --max-lines 800    # file-size line budget (0 disables)
straitjacket --max-nesting 4    # deep-nesting depth budget (0 disables)
```

- **`--max-lines`** — how long a file may be before `file-size` fires.
  Default 1500. `--max-lines 0` disables the rule outright.
- **`--max-nesting`** — how deeply a line may be indented before `deep-nesting`
  fires, measured off leading indentation. Default 8. `--max-nesting 0` disables
  the rule.

## Exempt paths from a rule

`file-size` and `stray-todo` take path prefixes in the
[config file](/docs/reference/config-file), which is usually tidier than
scattering markers through the files themselves:

```toml
file-size-exclude = ["notes/", "packages/generated/"]
todo-exclude = ["packages/legacy/"]
```

`color` has the same idea in reverse: `theme-files` names the files that are
*allowed* to define color literals, so your palette module stops being a finding.

```toml
theme-files = ["src/theme/tokens.css"]
```

## Change the output

```sh
straitjacket --format json    # machine-readable findings
straitjacket --format sarif   # SARIF 2.1.0 for code scanning
straitjacket --no-fail        # report everything but always exit 0
```

`--format json` is the right choice when another tool consumes the results.
`--no-fail` is what you want while adopting Straitjacket — you see every finding
without breaking the build, which matters because a first run against an
established repository usually turns up a lot.

## Make it stick

Rather than remember these flags for every run, commit a
[`straitjacket.toml`](/docs/reference/config-file) to the repo — the same
settings in one file, picked up automatically by every run and by CI:

```toml
# straitjacket.toml
skip = ["motion"]
max-lines = 800
theme-files = ["src/theme/tokens.css"]
file-size-exclude = ["notes/"]
```
