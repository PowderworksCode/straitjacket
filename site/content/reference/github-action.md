---
title: GitHub Action
description: The bundled composite Action and its typed inputs.
order: 4
---

The `PowderworksCode/straitjacket` GitHub Action installs the prebuilt binary and
runs it over your checked-out repository in one self-contained pass. It fails the
step on any error-level finding. For a walkthrough see
[Add Straitjacket to CI](/guides/ci); this page is the input reference.

## Usage

```yaml
permissions:
  contents: read
steps:
  - uses: actions/checkout@v5
  - uses: PowderworksCode/straitjacket@v{{version}}
    with:
      paths: "src tests"
      skip: "motion"
```

The tag on the `uses:` line is the pin. The Action installs the release it is —
`@v{{version}}` runs Straitjacket v{{version}} — so a release published later cannot apply
its new rules to a branch that changed nothing, and moving to a newer scanner is
a bump of that tag. The `version` input overrides it, and is only worth setting
to take a different release than the wrapper or to track `latest` on purpose.
See [Update Straitjacket](/guides/updating).

## Inputs

Every command-line option has an input, so a workflow configures the scan in YAML
rather than by assembling an argument string. Each input is optional; blanks fall
back to Straitjacket's own defaults, including a committed
[`straitjacket.toml`](/reference/config-file).

| input | default | meaning |
|-------|---------|---------|
| `version` | the Action ref | Release tag to install, such as `v{{version}}`, or `latest` to track releases. Unset, it is the release this Action ref ships. |
| `paths` | `.` | Files or directories to scan. |
| `only` | none | Run only these rules. |
| `skip` | none | Disable these rules. |
| `format` | `text` | Output written to the log — `text`, `json`, or `sarif`. |
| `max-lines` | config | Maximum lines per file. `0` disables `file-size`. |
| `max-nesting` | config | Maximum indentation depth. `0` disables `deep-nesting`. |
| `no-comments` | `false` | Enable the opt-in [`no-comments`](/reference/rules#no-comments-mode) rule. |
| `include-json` | `false` | Scan JSON files, which are skipped by default. |
| `no-ignore` | `false` | Scan what ignore files and the hidden-file convention exclude. |
| `config` | discovered | Use this configuration file instead of discovering one. |
| `no-config` | `false` | Ignore checked-in configuration. |
| `sarif-file` | none | Write a SARIF report to this path. Empty writes none. |
| `fail-on-findings` | `true` | Fail the step on error-level findings. |
| `fail-on-unused-markers` | `true` | Report suppression markers that suppress nothing. |
| `token` | none | Only needed while the repository is private. |

`paths`, `only`, and `skip` take either a list or a single line, so both of these
mean the same thing:

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

## Outputs

| output | meaning |
|--------|---------|
| `exit-code` | The Straitjacket exit code — `0` clean, `1` findings, `2` operational failure — so a later step can branch on the result even when `fail-on-findings` is off. |

## SARIF

The Action writes the SARIF file; **the upload is a step you add**, and that is
what needs `security-events: write`:

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

Set `fail-on-findings: "false"` on the scan step, or a failing scan ends the job
before the upload runs and you get the gate without the annotations. See
[SARIF / inline PR annotations](/guides/ci#sarif--inline-pr-annotations).

## Notes

- The Action is a **composite** action — it fetches a single static binary, so
  there's no toolchain or Node to set up.
- The `uses:` tag pins the scanner, so `version` needs setting only to depart
  from it. A ref that is not a release — `@main`, or a version bumped before its
  tag is pushed — installs the latest release and says so in a warning
  annotation.
- Set `fail-on-findings: "false"` to report findings without failing the build
  while you adopt Straitjacket.
