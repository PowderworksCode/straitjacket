<!-- straitjacket-allow-file:emoji — the sample scan output quotes an emoji finding, so the literal is what is being demonstrated -->

[Get started](/getting-started/) · [Read the docs](/guides/) · [Browse the rules](/reference/rules/) · [GitHub](https://github.com/PowderworksCode/straitjacket)

Straitjacket is a fast, deterministic scanner that looks for the weird stuff
LLMs produce. It's one static Rust binary and works with projects written in
any programming language.

<!--/hero-->

```sh
curl -fsSL https://straitjacket.dev/install | sh
```

Or build from source with `cargo install straitjacket`.

Run `straitjacket` at the root of any project. It honors your `.gitignore`,
prints one line per finding as `path:line:col [rule] matched`, and exits
non-zero on any error — so CI fails the moment slop lands.

Nine of the twelve rules are on at the first run, so the strictest
Straitjacket gets by default takes no configuration to reach. What you
disagree with, you turn off — `--skip` for a run, `straitjacket.toml` for
good, `straitjacket-allow` on the one line you meant. The other two you opt
into: `no-comments`, which is severe enough to ask for, and `test-quality`,
which downloads a grammar the first time it meets a test file.

```text
$ straitjacket

src/theme.ts:42:7  [color]  #1e1e1e
src/icons/Logo.tsx:12:5  [inline-svg]  <svg
docs/setup.md:3:1  [emoji]  🚀
src/api/handlers.ts:1:1  [file-size]  2214 lines
src/worker.ts:88:31  [deep-nesting]  nesting depth 9

straitjacket: 5 error(s), 0 warning(s) across 128 file(s); 0 suppressed
```

Encode your taste as deterministic checks, and run them across everything an
LLM writes.
