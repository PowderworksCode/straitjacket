# .githooks

Committed Git hooks that run the fleet's gate locally, before a commit lands,
so a failure surfaces here rather than after a push.

Fleet-managed by [conf](https://github.com/PowderworksCode/conf); edit them
there, not here — a local change is drift the next sync reports.

## Activate

Hooks are not enabled by a clone. The repository's development script does it,
or run it once yourself:

```sh
git config core.hooksPath .githooks
```

## What runs

- `commit-msg` enforces Conventional Commits on the subject line.
- `pre-commit` runs Straitjacket over the tree, through `run-straitjacket`.

`run-straitjacket` **fails** when Straitjacket is not installed. A hook that
skips its only check wherever the tool is missing reports "clean" most loudly
where it has looked least; the message says how to install it.

## Bypass

`git commit --no-verify` skips the hooks for one commit.
