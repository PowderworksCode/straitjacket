---
title: Add Straitjacket to CI
description: Run Straitjacket on every push with the bundled GitHub Action, and configure it with typed fields.
order: 1
---

Straitjacket is built to fail CI when it finds slop. The bundled GitHub Action
installs the prebuilt binary and runs it over your checked-out repository in one
self-contained pass, failing the step on any error-level finding.

## Minimal setup

Drop this in `.github/workflows/straitjacket.yml`:

```yaml
name: straitjacket
on: [push, pull_request]
jobs:
  scan:
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@v5
      - uses: PowderworksCode/straitjacket@v{{version}}
```

That's the whole thing. No toolchain to set up, no Node — the Action fetches a
single static binary and runs it.

> **The tag is the pin.**
> `@v{{version}}` selects the scanner as well as the Action wrapper, so this workflow
> runs Straitjacket v{{version}} until you change that line — a release published
> tomorrow cannot apply its new rules to a branch that changed nothing. Moving
> to a newer scanner is a bump of that tag, and the findings it turns up land in
> the pull request that bumps it. See [Update Straitjacket](/guides/updating).
> The newest release is on the
> [Releases page](https://github.com/PowderworksCode/straitjacket/releases).

## Configure it

Pass typed fields rather than a raw argument string. Each maps to a CLI flag:

```yaml
      - uses: PowderworksCode/straitjacket@v{{version}}
        with:
          paths: "src tests"       # default "."
          skip: "motion"
          only: ""                 # run only these rules
          max-lines: "800"         # file-size budget (0 disables)
          max-nesting: "6"         # deep-nesting budget (0 disables)
          format: "text"           # job-log format — text, json, or sarif
          no-comments: "false"
          include-json: "false"
          no-ignore: "false"
          fail-on-findings: "true"
          version: "latest"        # override the tag; see Update Straitjacket
```

`paths`, `only`, and `skip` accept either a single line or a YAML block list, so
both of these mean the same thing:

```yaml
        with:
          paths: src tests
          only: color,emoji
```

```yaml
        with:
          paths: |
            src
            tests
          only: |
            color
            emoji
```

A boolean input must be exactly `true` or `false`. `True` or `yes` is an error
rather than a silent `false`, because a scanner that quietly stops enforcing is
worse than one that fails.

Every field is optional; blanks fall back to Straitjacket's own defaults,
including any `straitjacket.toml` committed to the repo. The complete list is in
the [GitHub Action reference](/reference/github-action).

## Report without failing

To surface findings without failing the build — which is what you want while
adopting it — turn off the gate:

```yaml
      - uses: PowderworksCode/straitjacket@v{{version}}
        with:
          fail-on-findings: "false"
```

The Action sets an `exit-code` output (`0` clean, `1` findings, `2` operational
failure), so a later step can still branch on the result:

```yaml
      - id: scan
        uses: PowderworksCode/straitjacket@v{{version}}
        with:
          fail-on-findings: "false"
      - if: steps.scan.outputs.exit-code == '1'
        run: echo "findings present, not gating yet"
```

## SARIF / inline PR annotations

Straitjacket emits SARIF 2.1.0, which GitHub code scanning ingests and renders as
annotations on the PR diff. The Action writes the file; **you add the upload
step**, which is what needs the `security-events: write` permission:

```yaml
permissions:
  contents: read
  security-events: write

steps:
  - uses: actions/checkout@v5
  - uses: PowderworksCode/straitjacket@v{{version}}
    with:
      sarif-file: straitjacket.sarif
      fail-on-findings: "false"
  - uses: github/codeql-action/upload-sarif@v3
    with:
      sarif_file: straitjacket.sarif
```

`fail-on-findings: "false"` matters here: if the scan step fails the job, the
upload step never runs and you get the gate without the annotations. Let the
scan report, let the upload post, and gate afterwards on the `exit-code` output
if you want both.

Outside the Action, produce SARIF yourself:

```sh
straitjacket --format sarif --no-fail > straitjacket.sarif
```

`--sarif <path>` writes the report to a file *in addition to* whatever stdout
format you chose, which is handy when you want a readable log and a machine
artifact from one run.

## Other CI systems

There's no runtime dependency, so any CI works — install the binary and run it:

```sh
curl -fsSL https://straitjacket.dev/install | STRAITJACKET_VERSION=v{{version}} sh
straitjacket
```

The command exits non-zero on any error-level finding, which fails the job on
every CI system that respects exit codes. `STRAITJACKET_VERSION` is doing the
same job the Action's tag does here: without it the job takes whatever the
latest release is on the morning it runs, and updating becomes something that
happens to you rather than something you did.
