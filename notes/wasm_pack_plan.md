# Grammar packs over parsers: the plan

Written 2026-08-13, from the sjbrow session. Every number in this document was
measured on this box during that session; nothing is estimated. The prototype
benches live in the session scratchpad; the eyebrow rule itself is committed on
`eyebrow-rule` (946cca0, unpushed).

## Decisions taken

1. **The eyebrow rule ships in its strong form** — an element whose next
   element sibling is an `h1`-`h6`, holding literal text at or under
   `eyebrow-max-chars`, is a finding. Category labels and section numbering are
   flagged by design: the opinion is that a heading says its own name.
2. **Structural rules parse with treebank wasm packs**, not with tree-sitter
   crates linked as C.
3. **wasmer + cranelift is the runtime.** No wasmi fallback; environments that
   forbid executable pages are out of scope until one actually appears.
4. **GHCR/OCI is the registry**, for treebank grammar packs and infact fact
   packs alike, on the distribution scheme infact already wrote down in
   `docs/infact-packs.md`. The wasmer.io registry is not used.
5. **The contract is the pack ABI, not the engine**: WASI preview1 imports
   (three functions in practice) plus the `tb_*` exports. Anything honoring
   that contract runs a pack; wasmer is a choice inside it, not part of it.

## Why: the evidence

### The rule needs a parser

Measured across every frontend file on this box (3,067 HTML/CSS/JSX/TSX files,
44 heading sites hand-audited in the designed corpus):

| detector | recall | precision |
|---|---|---|
| same-line regex (all `RegexRule` can do) | 0% | 0% — 13 hits, all ordinal `<span>01</span>` false positives |
| two-line window | 86% | 75% — misses a `<Badge>` spread over three lines |
| element-sibling walk (parser) | 100% | 78% |

The ordinals are decisive: `<li><span>01</span><div><h3>` puts the span next
to the div, not the heading, and no line-based method can ever know that.

### Linked tree-sitter breaks the release

tree-sitter grammars are generated C. Both musl release targets fail in cc-rs
looking for `x86_64-linux-musl-gcc` / `aarch64-linux-musl-gcc` — exactly the
cross toolchain straitjacket's `.cargo/config.toml` exists to avoid needing.
`ci.yml` builds x86_64-musl on every run, so this fails CI, not just releases.
That is why `eyebrow-rule` is parked.

### The runtime ladder

One treebank pack (`treebank-tsx.wasm`, 1.12 MB, runtime + grammar + provenance
statically linked, already built `clang -O3` + `wasm-opt -all -O3`), one 696 KB
/ 18,800-line TSX file, parse time:

| runtime | parse | vs native | musl without a C toolchain |
|---|---|---|---|
| native tree-sitter | 32 ms | 1.0x | no — grammars are target C |
| **wasmer 7 + cranelift** | **50-52 ms** | **1.6x** | **yes — verified** |
| wasmtime 47 | 55 ms | 1.7x | no — unconditional `helpers.c` |
| wasmi 1.1 | 1.05-1.29 s | ~40x | yes |
| tinywasm 0.10 | 2.8 s | ~88x | yes |
| makepad-stitch 0.1 | segfault | — | — |

wasmi has no knob that changes this: eager translation, fuel, and host-side
caching were all tried and moved nothing. Interpretation is the cost itself.

### The wasmer verification

- `wasmer = { version = "7", default-features = false, features = ["sys", "cranelift"] }`
  cross-compiles to **both** musl targets under the existing rust-lld config
  with no C toolchain. Its `cc`/`cmake` build-deps are inert host libraries,
  invoked only by the feature-gated v8/jsc backends; `wasmer-vm` has no
  build.rs at all.
- The x86_64 **static-pie musl binary was executed**, not just linked: it JITs
  the pack and parses at 52 ms.
- **Correctness, not just speed**: the full s-expression of the same parse from
  wasmi and wasmer is byte-identical (7,267 bytes, diff clean).
- Per-module JIT compile is 90-160 ms, once per grammar per process.
  `Module::serialize` produces a 2.3 MB artifact that deserializes in 0.3 ms.
- Small files: 0.7 ms per 3.5 KB file (wasmi: 6.5 ms).

### The workload is smaller than it looks

A prefilter makes the parse cost nearly irrelevant for straitjacket: only 2 of
135 markup files in the real repos on this box contain a heading at all
(ordnung 1/11, website 1/3, fumadocs src 0/4, fumadocs-ui 0/117). Gating
`tb_parse` behind a heading regex skips ~98% of files, and straitjacket
already has the file text in hand.

## Architecture

```
GHCR (OCI artifacts)                      the registry: cache, not authority
 ├─ treebank grammar packs (.wasm)        provenance linked INSIDE the module
 └─ infact fact packs (JSON + manifest)   provenance in the manifest
        │
        ▼  digest-pinned pull, TOML lock
local content-addressed cache
 ├─ pack.wasm                by sha256
 └─ pack.cranelift-artifact  keyed (pack sha256, wasmer version, target)
        │                    derived locally, NEVER published
        ▼
consumer (straitjacket, anything else)
 └─ wasmer/cranelift engine → WASI + tb_* ABI → findings
```

Principles carried over from infact's design, which treebank adopts rather
than re-invents: the registry is a prebuilt cache and the authority is the
derivation inputs; local, private-registry, and public-registry artifacts share
one manifest and layout; resolution is digest-locked and works offline;
publication is always explicit. Treebank packs satisfy the cache-not-authority
doctrine natively because provenance (upstream, sha, patches, toolchain pins)
is linked into the `.wasm` itself.

## Workstreams

### treebank (tbwasm session owns the pack side)

1. **HTML grammar pack.** The critical-path item: packs cover JS/TSX today,
   and the eyebrow rule needs HTML. Vue and Svelte remain gaps; the rule
   already treats "no grammar" as "not read", visibly, in one function.
2. **Publish packs to GHCR as OCI artifacts**, replacing "nothing has been
   published anywhere". Reuse the byte-reproducibility discipline as the
   publish gate: re-derive, compare, then push.
3. **Registry/cache client** — the resolution order, the digest lock, the
   derived-artifact cache keyed by (pack digest, wasmer version, target).
   Template: `infact/docs/infact-packs.md`.
4. Optional, second-order: a batch `tb_tree_dump` export (one call, packed
   node records, one memory read) for interpreter hosts. Under a JIT the
   per-call walk is already cheap; do this only if the proposed query API
   lands anyway.

### straitjacket (this repo, `eyebrow-rule` branch)

1. Drop `tree-sitter`, `tree-sitter-html`, `tree-sitter-typescript`; add
   pinned wasmer (sys + cranelift, default features off).
2. Rehost the sibling walk in `src/rules/eyebrow.rs` onto the `tb_*` ABI. The
   rule's semantics, config keys, and all 22 tests stay identical — identical
   trees were proven, so findings cannot change. The dynamic-expression check
   (`{icon}` is an icon slot, not an eyebrow) must be ported for real; the
   session probe shortcut it.
3. Pack resolution: a `grammar-packs` path in `straitjacket.toml` first
   (vendored or locally built packs), the registry client when treebank ships
   it. Digest verification on load either way.
4. One engine per process; one `Module` per grammar, cached as a cranelift
   artifact in the XDG cache; one instance reused across files with
   `tb_tree_free` per file.
5. The heading prefilter, measured again at port time.
6. Gates: the usual three, plus the x86_64-musl release build that killed the
   linked-C approach, plus straitjacket clean on itself.

### infact

Nothing changes. Its pack design is the template; the convergence is treebank
adopting the same scheme, not infact moving.

## Sequencing

```
treebank: html grammar ──► publish tsx+html packs to GHCR ──► registry client
                                                                    │
straitjacket: port eyebrow to wasmer host (start now, local packs) ─┤
                                                                    ▼
                                        unpark eyebrow-rule, CI green, ship
```

The straitjacket port does not wait: it can build today against packs copied
from treebank's `dist/wasm/`, switching to registry resolution when that
exists. The TSX half of the rule is testable immediately; the HTML half turns
on when the HTML pack lands.

## Costs and risks

| item | position |
|---|---|
| wasmer dependency weight | 207 crates, bench binary 11.8 MB (straitjacket today: 3.2 MB, with linked tree-sitter 4.5 MB). Accepted. |
| JIT needs W^X pages | Accepted; no fallback engine by decision. Fail loudly with a clear error if mmap-exec is denied, never silently skip the rule. |
| `proc-macro-error2` in wasmer's tree | rustc already warns it will be rejected by a future release. Pin wasmer; track before each toolchain bump. |
| wasmer version bumps | Invalidate cached cranelift artifacts only; the cache key includes the version, so the cost is one re-JIT (~100 ms per grammar), never wrong results. |
| pack ABI evolution | `tb_pack_abi` is versioned in the shim; the host checks it on load and refuses mismatches. |
| eyebrow rule residual false positives | A CTA link and a text-bearing logo above heroes, measured 2 in ~46k files; the rule is opinionated by design and suppression markers exist. |
| Vue/Svelte not covered | Known gap, held in one function (`eyebrow::grammar`), closes when treebank grows those grammars. |
