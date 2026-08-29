---
title: Straitjacket
description: A secret scanner, but for slop.
tab-title: Straitjacket by Powderworks
---

<!-- straitjacket-allow-file:emoji — the sample scan output quotes an emoji finding, so the literal is what is being demonstrated -->

<p class="cover"><img src="/cover.jpg" alt="Engraving of a patient restrained in a strait-waistcoat" width="220"><span class="credit">An 1838 engraving of a <a href="https://en.wikipedia.org/wiki/Straitjacket">strait-waistcoat</a>. <a href="https://wellcomecollection.org/works/dh2xhuph">Wellcome Collection</a>, <a href="https://creativecommons.org/publicdomain/mark/1.0/">Public Domain Mark</a>.</span></p>

**A secret scanner, but for slop.**

[Get started](/getting-started/) · [Read the docs](/guides/) · [Browse the rules](/reference/rules/) · [GitHub](https://github.com/PowderworksCode/straitjacket)

Straitjacket is a fast, deterministic scanner that flags the weird code and
text LLMs produce. It sweeps your files against a set of
snobby-but-configurable rules and flags anything it finds — one static Rust
binary, no runtime, so it drops into any repo's CI regardless of language or
stack.

```sh
curl -fsSL https://raw.githubusercontent.com/PowderworksCode/straitjacket/main/install.sh | sh
```

Or build from source with `cargo install straitjacket`.

## What it catches

Everything is on by default — Straitjacket runs at its max, and you ratchet down
with `--skip`. Each rule only looks at the file types where it makes sense.

- **Emoji & AI tells** — emoji glyphs in code, comments, strings, and Markdown;
  one of the most reliable giveaways that a machine wrote it.
- **Hardcoded colors** — raw hex and CSS color functions that should be a theme
  token, not sprinkled inline.
- **Oversized files** — the 1,500-line monsters that pass review one screen at
  a time and sneak up on you.
- **Deep nesting** — logic indented past the depth budget, read straight off
  the indentation, so it works in any language without a parser.
- **Stray TODOs** — TODO, TBD, FIXME and WIP markers left behind in comments.
- **Design drift** — inline SVG, literal font stacks, and ad-hoc transitions;
  the details that quietly escape a design system.

## One command. Every rule.

Run `straitjacket` at the root of any project. It honors your `.gitignore`,
prints one line per finding as `path:line:col [rule] matched`, and exits
non-zero on any error — so CI fails the moment slop lands.

No config to write, no toolchain to install. Suppress a false positive on one
line with `straitjacket-allow`, or a whole file with `straitjacket-allow-file`.

```text
$ straitjacket

src/theme.ts:42:7  [color]  #1e1e1e
src/icons/Logo.tsx:12:5  [inline-svg]  <svg
docs/setup.md:3:1  [emoji]  🚀
src/api/handlers.ts:1:1  [file-size]  2214 lines
src/worker.ts:88:31  [deep-nesting]  nesting depth 9

straitjacket: 5 error(s), 0 warning(s) across 128 file(s); 0 suppressed
```

Encode your taste as deterministic checks and run them across everything an
LLM writes — so you never have to go "Yuck!" by hand again.
