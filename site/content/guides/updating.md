---
title: Update Straitjacket
description: Move to a new release on your machine and in CI, roll one back, and keep the two in step.
order: 2
---

Straitjacket is a gate, so an update is never only an update: a new release can
carry a new rule, and a new rule turns findings up in code nobody touched. That
is the whole design problem, and the answer this project takes is the ordinary
one — **your machine updates when you say so, and CI updates when you merge the
bump.** Nothing here checks for a new version behind your back, and nothing
installs one.

## What am I running?

```sh
straitjacket --version
```

## Update the binary

Re-run the installer. It is the update path — there is no separate command,
because there is nothing a second one would do differently:

```sh
curl -fsSL https://straitjacket.dev/install | sh
```

It resolves the latest release, verifies the download against that release's
`SHA256SUMS`, and replaces the binary by rename, so a Straitjacket that is
running at the time is swapped rather than written through. Installing over an
older copy is the supported path and needs no uninstall.

From source, `cargo install` needs to be told that overwriting is the point:

```sh
cargo install straitjacket --force --locked
```

`--locked` builds against the dependency versions the release was tested with.
Without it Cargo is free to pick newer ones, which is a different binary than
anybody else's.

If Straitjacket arrived some third way — a package manager, a vendored copy in
an image — update it that way instead. The installer would write a second copy
to `~/.local/bin` and which one you get would come down to `PATH` order.

## Pin or roll back

`STRAITJACKET_VERSION` takes a tag, and the installer will move you backwards as
readily as forwards:

```sh
curl -fsSL https://straitjacket.dev/install | STRAITJACKET_VERSION=v0.1.2 sh
```

That is the answer to a release that turns up a finding you are not ready to
deal with today. It is also the honest short-term fix while you work through
one — better than `--skip`, which quietly turns the rule off everywhere and
tends to stay.

## Update CI

The tag on the `uses:` line is the whole pin. It selects the scanner as well as
the Action wrapper, so updating CI is a one-line change to a workflow file:

```diff
-      - uses: PowderworksCode/straitjacket@v0.1.2
+      - uses: PowderworksCode/straitjacket@v{{version}}
```

Let Dependabot write it, and the bump arrives as a pull request with the new
scanner's findings already on it — which is exactly where you want to see them,
rather than on somebody else's unrelated branch:

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: github-actions
    directory: /
    schedule:
      interval: weekly
```

A bump whose checks go red is not a broken update. It is the new release
telling you what it now flags, in a pull request that exists to answer that
question. Read [the changelog](https://github.com/PowderworksCode/straitjacket/blob/main/CHANGELOG.md),
then fix, [suppress](/guides/suppressing-findings), or
[tune](/guides/tuning-rules) — and if none of those is today's job, revert the
bump and come back to it.

To adopt a release without gating on it first, land the bump with the gate off
and read the report for a while:

```yaml
      - uses: PowderworksCode/straitjacket@v{{version}}
        with:
          fail-on-findings: "false"
```

### Tracking releases instead

If you would rather take new rules as they ship, ask for that by name:

```yaml
      - uses: PowderworksCode/straitjacket@v{{version}}
        with:
          version: "latest"
```

It is a reasonable choice for a repository you are actively grooming, and a bad
one for a gate that other people's branches have to pass. The difference is
whether a red check is something a person can act on.

## Keep your machine and CI in step

Findings that only appear in CI are the expensive kind: they surface after a
push, on someone else's schedule. The fix is not clever — run the same release
in both places. Take the tag out of the workflow and hand it to the installer:

```sh
curl -fsSL https://straitjacket.dev/install | STRAITJACKET_VERSION=v{{version}} sh
```

Then a scan that is clean before you push is clean after.
