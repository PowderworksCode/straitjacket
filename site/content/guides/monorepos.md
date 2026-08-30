---
title: Scope checks per package in a monorepo
description: Give each package its own settings, and scan one package or the whole tree.
order: 6
---

Every rule in Straitjacket looks at **one file at a time**. Nothing compares a
file against the rest of the tree, so a monorepo needs no boundary markers and no
special mode — a scan from the root is just a scan of every file under it.

What a monorepo does want is *different settings per package*, and that falls out
of how configuration is discovered.

## One config per package

Straitjacket looks for `straitjacket.toml` in the current directory and walks
**up** to the filesystem root, using the first one it finds. So a package with
its own file gets its own settings when you scan from inside it:

```
straitjacket.toml                  # repo-wide defaults
packages/web/straitjacket.toml     # stricter: the design system lives here
packages/legacy/straitjacket.toml  # looser: grandfathered
```

```sh
cd packages/web && straitjacket
```

That picks up `packages/web/straitjacket.toml` and never sees the root one. The
files do not merge — the nearest config wins outright — so a package config
should be complete rather than a delta.

```toml
# packages/legacy/straitjacket.toml
skip = ["motion", "inline-font"]
max-lines = 4000
```

## Scanning the whole tree

Run from the root and you get the root config for everything:

```sh
straitjacket
```

Per-package configs are *not* consulted in that pass — discovery starts from the
working directory, not from each file. If you want each package checked under its
own rules, scan them separately, which in CI is a matrix job or a loop:

```sh
for pkg in packages/*/; do (cd "$pkg" && straitjacket) || fail=1; done
```

## Narrowing from the root

For the common case — one root config, but only some directories worth scanning —
set `paths` and skip the loop entirely:

```toml
# straitjacket.toml
paths = ["packages/web/src", "packages/api/src"]
```

Or pass them per-run:

```sh
straitjacket packages/web packages/api
```

## Excluding by prefix

Two rules take path prefixes directly, which is usually cleaner than carving up
the scan:

```toml
file-size-exclude = ["packages/generated/", "notes/"]
todo-exclude = ["packages/legacy/"]
```

The [suppression markers](/reference/suppression-markers) still cover the
one-off exceptions that no amount of scoping should.
