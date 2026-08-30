---
title: Getting started
description: Install Straitjacket, run your first scan, read the output, and handle a finding — end to end.
order: 1
---

This tutorial takes you from nothing to a clean scan. By the end you'll have
Straitjacket installed, understand what it printed, and know how to deal with a
finding — either by fixing it or by telling Straitjacket it's fine.

## 1. Install

Grab the prebuilt binary (Linux x86_64, macOS arm64/x86_64):

```sh
curl -fsSL https://straitjacket.dev/install | sh
```

It verifies the download against the release checksums and installs to
`~/.local/bin`. Prefer to build from source? `cargo install straitjacket`.
Full details — install location overrides, pinning a version — are in the
[installation reference](/reference/cli#installing). Running that same command
again later is how you [update](/guides/updating).

Check it's on your `PATH`:

```sh
straitjacket --version
```

## 2. Run your first scan

From the root of any project:

```sh
straitjacket
```

With no arguments, Straitjacket scans the current directory, honoring your
`.gitignore`. Nine of the eleven rules are on by default — it runs near its
max and you ratchet down later. The other two are modes you opt into:
`no-comments`, and `test-quality`, which parses your tests with a downloaded
grammar.

If everything is clean you'll see:

```
straitjacket: ok — no findings in 128 file(s)
```

## 3. Read the output

More likely, it found something. Each finding is one line:

```
src/theme.ts:42:7  [color]  #1e1e1e
```

Findings usually carry an indented message and a `help:` line under them:

```
src/theme.ts:42:7  [color]  #1e1e1e
  hardcoded color literal
  help: use a theme token or CSS variable
```

That reads as `path:line:col  [rule]  matched` — the file and position, the rule
that fired, and the exact text that tripped it. At the end you get a summary:

```
straitjacket: 5 error(s), 0 warning(s) across 128 file(s); 0 suppressed
```

The process exits **1** when there's any error-level finding — that's what makes
[CI](/guides/ci) fail — **0** when it's clean, and **2** if the run itself
failed (a bad config, an unknown rule id).

> **Expect a lot on the first run**
> Straitjacket runs every rule at its max, so pointing it at an existing
> repository for the first time will usually turn up dozens of findings. That's
> the tool working, not a misconfiguration. Add `--no-fail` to read the report
> without failing your shell, and ratchet from there:
> 
> ```sh
> straitjacket --no-fail
> ```

Want to see what each rule means? List them:

```sh
straitjacket --list-rules
```

Or read the full [rules reference](/reference/rules).

## 4. Handle a finding

You have two honest choices for any finding.

**Fix it.** Most findings point at something real — a hardcoded color that
should be a theme token, a 2,000-line file that wants splitting, an emoji that
snuck into a comment. Change the code and re-run.

**Suppress it.** Sometimes the finding is a legitimate exception — a palette file
that's *supposed* to be full of hex codes. Add a marker comment on the line:

```ts
const brandColor = "#ff6600"; // straitjacket-allow: fixed brand color, not themeable
```

Or exempt a whole file by putting a marker on any one line of it:

```css
/* straitjacket-allow-file:color  design tokens live here */
:root { --bg: #1e1e1e; --fg: #abb2bf; }
```

The full scoping rules are in [Suppress a false positive](/guides/suppressing-findings).

## 5. Scope the scan

As you go, you'll want to run a subset:

```sh
straitjacket src tests          # only these paths
straitjacket --only emoji,color # only these rules
straitjacket --skip motion      # everything except this rule
```

## Where to next

- Wire Straitjacket into CI so it runs on every push → [Add Straitjacket to CI](/guides/ci)
- Move to a new release, here and in CI → [Update Straitjacket](/guides/updating)
- Save your settings in the repo → [Config file](/reference/config-file)
- See every flag and its default → [CLI reference](/reference/cli)
