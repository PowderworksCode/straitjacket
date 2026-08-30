---
title: Background & philosophy
description: Why Straitjacket exists, and the personal taste it encodes into deterministic checks.
order: 1
---

Straitjacket started life as a series of per-repo `lint-*` scripts, written
because I got annoyed with the way Claude kept messing with the design of the interface, as
well as with the kinds of code and text it would output. I'd written versions of
these linters across various projects over the last few years, and I kept finding
new smells as I generated more code and text over time. Eventually I decided to
bundle them all into one tool, so I wouldn't have to keep rewriting them
haphazardly all over the place — and so other people could use it and tell me what
other annoying things LLMs tend to do.

During the initial development of Straitjacket, I had a strong realization: what
bothers me most about the way LLMs change the design of an application maps neatly
onto common UI settings. Claude randomly inserts elements and changes their colors
— that's the province of a theme switcher. Claude decides it needs ten font
families and a hundred sizes and weights — that's the purview of a font family and
size picker. Every element on a page wiggles in its own individual way; well,
well, well, that's a motion-control toggle. So, in a way, in lieu of guidance — of
an enforced design system — why shouldn't Claude get freaky with it? We never said
it couldn't.

So, alongside restricting the design tokens (colors, fonts, motion) to blessed
files, I'd recommend giving users a way to control these settings too. To me, the
two go hand in hand. Likewise, when reviewing code, I found it was very easy for
Claude to squirrel thousands of lines away into a single file. I'd review all the
lines, they'd look fine, but these monsters would sneak up on me before I knew it.
Refactoring them always made the codebase better, and I've found that 1500 lines
is about where they start breaking down logically enough for me to notice.

As for slop text, it just smells. There's no way around it, and I don't like it.
Straitjacket does its best to scan for the most common signs. It's not wrong to
use the word *delve*, but it does get suspicious when you use it often, alongside
other signs. Not trying to get too fancy with it.

Straitjacket has become an exercise in me encoding as much of my personal tastes
as I can into deterministic checkers I can run across LLM output, hopefully saving
me the trouble of catching each one myself.
