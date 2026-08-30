---
title: About
description: What Straitjacket is, who builds it, and why it exists.
order: 2
---

Straitjacket is a fast, deterministic scanner that flags the weird code and
text LLMs produce: emoji glyphs in source files, hardcoded colors that should
be theme tokens, sprawling thousand-line files, logic indented past the depth a
reader can hold, and deferred-work markers like `TODO` left behind in comments.
It is one static Rust binary with no runtime dependencies, so it drops into any
repository's CI regardless of language or stack.

Every rule runs on the text of a file — the same binary works across every
language and stack, no toolchain required. The trade is that findings are
pattern-level: Straitjacket tells you what is in the source, not what it means.
The reasoning behind the rule set is written down in
[the project philosophy](/project/philosophy/).

Straitjacket is built by [The Powderworks Agentic Coding
Consortium](https://powderworks.dev), and maintained by
[Zack](https://github.com/zmaril). The code is MIT-licensed and lives on
[GitHub](https://github.com/PowderworksCode/straitjacket), where releases,
checksums, and the full commit history are public.

If you want to shape what it catches — a new smell, a false positive you should
not have to live with — the [contributing guide](/project/contributing/) is
the way in.

## Newsletter

Releases, and notes on building software with agents.

<iframe class="embed" title="Subscribe to the Powderworks newsletter" src="https://newsletter.powderworks.dev/embed" scrolling="no"></iframe>
