---
title: Suppression markers
description: Exact syntax and matching semantics for straitjacket-allow and straitjacket-allow-file.
order: 5
---

Straitjacket has two escape hatches, both plain marker text. It matches the text
anywhere on a line — the surrounding comment syntax (`//`, `#`, `/* */`,
`<!-- -->`) doesn't matter, so use whatever's valid in the file. For a
step-by-step walkthrough see [Suppress a false positive](/docs/guides/suppressing-findings).

## Line scope

A marker on a line suppresses findings **on that same line**.

| marker | effect |
|--------|--------|
| `straitjacket-allow` | suppress **every** rule on this line |
| `straitjacket-allow:<rule>` | suppress only `<rule>` on this line |

```ts
const brandColor = "#ff6600"; // straitjacket-allow:color  brand color, not themeable
```

Text after the marker is free-form — use it to record why.

## File scope

A marker on **any one line** of a file exempts the whole file. The top of the
file is conventional.

| marker | effect |
|--------|--------|
| `straitjacket-allow-file` | exempt **every** rule for the whole file |
| `straitjacket-allow-file:<rule>` | exempt only `<rule>` for the whole file |

```css
/* straitjacket-allow-file:color  design tokens — colors live here */
:root { --bg: #1e1e1e; --fg: #abb2bf; }
```

## Choosing a scope

- Prefer the **rule-scoped** form (`straitjacket-allow-file:color`) over the
  blanket one — it keeps every *other* rule live on the file.
- **Whole-file rules need file scope.** `file-size` doesn't attach to a single
  source line, so a per-line `straitjacket-allow` won't silence it; use
  `straitjacket-allow-file:file-size`.
- `<rule>` is any rule id from the [rules reference](/docs/reference/rules) — the
  same ids `--only` and `--skip` accept.
