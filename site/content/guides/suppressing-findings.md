---
title: Suppress a false positive
description: Silence a finding on one line or across a whole file with straitjacket-allow markers.
order: 3
---

When a finding is a legitimate exception — a palette file that's supposed to be
full of hex codes, a generated file that's supposed to be huge — you tell
Straitjacket to leave it alone with a marker comment.

There are two scopes. Both just look for the marker text, so the comment syntax
(`//`, `#`, `/* */`, `<!-- -->`) doesn't matter — use whatever's valid in the
file.

## One line

Add a same-line comment:

```ts
const brandColor = "#ff6600"; // straitjacket-allow: fixed brand color, not themeable
```

- `straitjacket-allow` suppresses **every** rule on that line.
- `straitjacket-allow:<rule>` suppresses only that rule, e.g. `straitjacket-allow:color`.

Anything after the marker is free-form — use it to note *why*.

## A whole file

Put the marker on any one line of the file (the top is conventional). This is the
right tool for a theme or palette file full of legitimate hexes:

```css
/* straitjacket-allow-file:color  design tokens — colors live here */
:root { --bg: #1e1e1e; --fg: #abb2bf; }
```

- `straitjacket-allow-file` exempts **every** rule for the file.
- `straitjacket-allow-file:<rule>` exempts only that rule for the file — so the
  palette above still gets checked for emoji, oversized length, and everything
  else.

## Which scope for which rule

**`file-size`** is inherently whole-file — it doesn't attach to a single line, so
a per-line `straitjacket-allow` won't silence it. You need
`straitjacket-allow-file:file-size`, or better, one of the path-prefix exclusions
in [Exclude big or generated files](/guides/ignoring-files).

**`emoji`** is the other common file-scope case: a fixture module that
deliberately exercises Unicode will trip it on every line, and
`straitjacket-allow-file:emoji` says that once instead of everywhere.

Prefer the **rule-scoped** form (`straitjacket-allow-file:color`) over the
blanket one whenever you can — it keeps every *other* rule live on that file,
which is almost always what you want.

For the exact matching semantics, see the
[suppression-markers reference](/reference/suppression-markers).
