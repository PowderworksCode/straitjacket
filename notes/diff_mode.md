# Rules about the shape of a change

Straitjacket's rules all read a file's final state. Some slop does not survive
into the final state: a comment rewritten in place leaves a file that reads
perfectly well, and only the edit is odd. This note records what already exists
in that space, what the prototype measures, and what is worth building.

## What exists

**Scoping a linter to the diff.** `reviewdog` runs any linter and drops findings
that fall outside the change, with three filter modes: added lines, added lines
±N of context, or whole modified files. `diff-cover` and `git-diff-lint` do the
same for coverage and for lint respectively. All of them answer "is this finding
*on* a changed line", which is a different question from "is this *change* odd",
and none of them would see a comment rewritten in place.

**Rules about the pull request.** `danger` / `danger-js` is the closest existing
thing. It exposes `git.modified_files`, `git.created_files`, `git.deleted_files`,
`git.diffForFile`, `git.structuredDiffForFile` (hunks), `git.linesOfCode`, and
JSON-patch views of structured files, and you write rules in JS against them.
What it does not have is any model of what a comment is: there is no
comment-only or whitespace-only predicate anywhere in the API or in the plugin
set. A Danger rule for this would be a hand-rolled per-language comment scanner
in JavaScript, which is precisely the part that is hard.

**Structural diff.** `difftastic` parses both sides with tree-sitter and diffs
the trees; `gumtree` (Falleri et al.) and `RefactoringMiner` do the same for
research on move and rename detection. Difftastic is the only one with a usable
predicate: `difft --ignore-comments --check-only --exit-code` exits 0 when the
two sides are syntactically identical once comments are set aside. That is a
real comment-only detector and it is measured against the prototype below. Its
limit is definitional rather than technical: `--ignore-comments` also ignores
formatting, so it cannot distinguish "a comment changed" from "rustfmt ran".

**The academic name.** A comment change committed without an accompanying code
change is an **Independent Comment Change (ICC)**, named and measured in Wang,
He, Pal, Marinov and Zhou, *Suboptimal Comments in Java Projects: From
Independent Comment Changes to Commenting Practices*, TOSEM 32(2), 2023 — 24
million comment changes over 4,392 Java repositories. Their thesis is that an
ICC is evidence the comment was *previously wrong*, so ICCs are a proxy for
suboptimal comments. The construct is identical to the observation being chased
here; the reading is the opposite, and it is worth holding both. Related:
*Comments on Comments: Where Code Review and Documentation Meet* (arXiv
2204.00107) reports that reviewers comment on a diff chunk 50.8% of the time
when the contributor also touched a comment, against 15.8% for code-only chunks.
Something about a comment change already draws human attention.

**AI-PR writing.** CodeRabbit ships slop detection for public PRs but publishes
no signals. The useful result is arXiv 2605.02273, which finds that structural
features of a change — added and deleted lines, files touched, entropy — predict
review burden far better than anything semantic (AUC 0.957 for structural
features alone). That is direct evidence for the thesis that the shape of a
change carries the information. The markers human reviewers reported using are
all final-state ones: emoji in comments, step-by-step commenting, verbose style,
Unicode artifacts — which is the set Straitjacket already covers.

**What nobody is doing.** There is no fast, deterministic, multi-language linter
whose rules take the change as their input and are about the change's shape.
reviewdog scopes, Danger supplies primitives without a language model,
difftastic renders, ICC is a paper with a Java-only artifact. The gap is real.

## What the prototype measures

`diff::comment_only_change` cuts every comment out of both sides using the
existing `rules::comments` scanner and compares what is left. If the remaining
code lines are identical the change touched nothing but comments. Blank lines
and leading whitespace are kept, so reindenting or reflowing code is a code
change; a line a comment leaves empty is dropped, so adding a whole comment line
is not.

Measured with `examples/diff_probe` over every non-merge commit:

| corpus | commits | modified files | comment-only files | pure-comment commits |
|---|---|---|---|---|
| `~/powderworks/*` (14 repos) | 842 | 5,510 | 33 (0.60%) | 2 (0.24%) |
| ripgrep + express | 7,903 | 10,304 | 381 (3.70%) | 181 (2.29%) |

All 33 powderworks hits were read by hand. Every one is a deliberate,
well-motivated change: renames carried into prose (`ledger.json` →
`ledger.toml`, `sync-linguist.py` → `langbank-sync`), `straitjacket-allow`
suppressions being added, a note rewritten to match a code change made elsewhere
in the same commit. **None is odd.** On the two external repositories the
dominant class is typo fixes and doc-link migrations — also benign.

Two refinements were measured. Requiring the comment's new vocabulary to appear
nowhere else in the commit ("tracks nothing") cuts 33 → 3 and 381 → 164.
Additionally requiring a rewrite to have replaced most of its vocabulary cuts
those to 1 and 32. The survivors are still benign, and short comments make the
vocabulary test unreliable — a one-word typo fix scores as a total rewrite.

## What a parser buys

Every one of the 5,510 powderworks file changes was cross-checked against
`difft --ignore-comments --check-only --exit-code`.

| | difftastic: comment-only | difftastic: changed |
|---|---|---|
| **straitjacket: comment-only** | 32 | 1 |
| **straitjacket: changed** | 37 | 5,440 |

Agreement 99.31%, with nothing difftastic failed to parse. Both disagreement
cells favour the cheap scanner:

- The **37** are all `rustfmt` reflows, concentrated in six formatting sweeps.
  Difftastic is right that there is no syntactic change, but they are not comment
  changes. `--ignore-comments` merges two categories that want to stay apart.
- The **1** is `straitjacket 6db8f0c2`, a pure `///` doc-comment addition.
  Straitjacket is right and difftastic is wrong; difftastic does not treat Rust
  doc comments as comments.

**A parser buys nothing for this question on this evidence.** The cheap scanner
already handles the cases a parser would, and it draws the line in the more
useful place. Treebank's wasm packs are not needed here. Where a parser would
matter is a *different* rule — "a docstring changed while its function did not"
needs to know which function a comment attaches to, and no comment scanner can
answer that.

## The verdict on the seed rule

Precise, and it finds nothing. On the corpus that prompted the observation the
rule fires 33 times with zero genuine hits; on ordinary human repositories it
fires on 3.7% of modified files, and those are typo fixes. It should not go
anywhere near the default set. It is a real, cheap, low-noise diff-shape
predicate that belongs behind an opt-in, and its interest is as a building block
for the ranked candidates rather than as a rule on its own.
