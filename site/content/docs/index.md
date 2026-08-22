---
title: Introduction
description: What Straitjacket is, where everything lives in the docs, and the fastest way to a first scan.
order: 0
---

Straitjacket is a fast, deterministic scanner that flags the weird code and text
LLMs produce. It sweeps your files against a set of rules — with snobby yet
configurable defaults — and flags anything it finds. It's a single static Rust
binary with no runtime dependencies, so it drops into almost any environment or
repo's CI, regardless of language or stack.

If you're new here, start with the [Getting started
tutorial](/docs/getting-started): install, first scan, first finding, end to
end. From there:

<Cards>
  <Card
    title="How-to guides"
    href="/docs/guides/ci"
    description="Recipes for specific jobs: wire it into CI, suppress a false positive, exclude generated files, tune the rules."
  />
  <Card
    title="Reference"
    href="/docs/reference/rules"
    description="Every rule, every CLI flag, every Action input, and the exact suppression-marker syntax."
  />
  <Card
    title="About"
    href="/docs/about/philosophy"
    description="Background & philosophy, how to contribute a new smell, and the license."
  />
</Cards>

## In a hurry?

```sh
# install (Linux x86_64/aarch64, macOS arm64/x86_64)
curl -fsSL https://raw.githubusercontent.com/PowderworksCode/straitjacket/main/install.sh | sh

# or, from source
cargo install straitjacket

# scan the current directory (honors .gitignore)
straitjacket
```

Everything is on by default — Straitjacket runs at its max, and you ratchet down
with `--skip`. A first run against an established repository will find a lot;
add `--no-fail` to read the report without failing your shell. See the
[Getting started tutorial](/docs/getting-started) for a guided
walkthrough.
