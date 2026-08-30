---
title: Contact
description: "How to reach the Straitjacket maintainers: bug reports, rule proposals, and security issues."
order: 4
---

Straitjacket has no support inbox; GitHub is the channel. Everything happens in
public except security reports, which have a private route.

## Bug reports and feature requests

Open an issue on the repository. Include the version (`straitjacket --version`),
the command you ran, and a minimal file that reproduces the finding.
[GitHub Issues](https://github.com/PowderworksCode/straitjacket/issues).

## Proposing a new rule

New smells start as issues too — describe the pattern, show real examples of
it, and say why it signals machine-written content. The
[contributing guide](https://github.com/PowderworksCode/straitjacket/blob/main/CONTRIBUTING.md)
has what makes a rule land.

## Security problems

Anything exploitable — crashes on hostile input, path traversal in the walker —
should be reported privately via
[GitHub Security Advisories](https://github.com/PowderworksCode/straitjacket/security/advisories/new)
rather than a public issue.
