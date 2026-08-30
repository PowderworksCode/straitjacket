# straitjacket

<p align="center">
  <img src="assets/strait-waistcoat.jpg" alt="Engraving of a patient restrained in a strait-waistcoat" width="320">
</p>

<p align="center">
  <a href="https://crates.io/crates/straitjacket"><img src="https://img.shields.io/crates/v/straitjacket.svg" alt="straitjacket on crates.io"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT licensed"></a>
  <a href="https://straitjacket.dev"><img src="https://img.shields.io/badge/docs-straitjacket.dev-informational" alt="Documentation at straitjacket.dev"></a>
</p>

Straitjacket is a fast, deterministic scanner that flags the weird code and text
LLMs produce. It sweeps your files against a set of configurable rules and flags
anything it finds. One static binary, no project setup, any language.

```sh
# Linux x86_64/aarch64, macOS arm64/x86_64
curl -fsSL https://straitjacket.dev/install | sh
straitjacket
```

```text
src/theme/button.css:12:14  [color]  #ff6600
  hardcoded color literal
  help: use a theme token or CSS variable
straitjacket: 1 error(s), 0 warning(s) across 84 file(s); 0 suppressed
```

Or build from source with `cargo install straitjacket`. In CI, the bundled
Action installs the binary and scans the checked-out repository:

```yaml
- uses: PowderworksCode/straitjacket@v0.1.3
```

## Documentation

Everything lives at **[straitjacket.dev](https://straitjacket.dev)**:

- [Getting started](https://straitjacket.dev/getting-started/) — install, first scan, first finding
- [Guides](https://straitjacket.dev/guides/) — CI, monorepos, tuning rules, suppressing findings
- [Rules](https://straitjacket.dev/reference/rules/) — what each rule flags and when it runs
- [CLI](https://straitjacket.dev/reference/cli/) — every flag, exit code, and output format
- [Configuration](https://straitjacket.dev/reference/config-file/) — `straitjacket.toml` keys and defaults
- [GitHub Action](https://straitjacket.dev/reference/github-action/) — the typed inputs
- [Background & philosophy](https://straitjacket.dev/project/philosophy/) — why this exists

## Contributing

The most useful thing you can send is a concrete example — a pattern
Straitjacket should catch, or a false positive it shouldn't. `scripts/dev.sh`
stands up everything you need: the Rust workspace, and the site dependencies the
documentation tests run against. See [CONTRIBUTING.md](CONTRIBUTING.md) for what
makes a good rule and how to run the tests. Release-to-release changes are in [CHANGELOG.md](CHANGELOG.md).

## License

Code is [MIT](LICENSE).

The banner image (`assets/strait-waistcoat.jpg`) — *Insane patient in a
strait-waistcoat*, [Wellcome Collection](https://wellcomecollection.org/works/ckwscya3)
(L0011301) — is licensed [CC BY 4.0](https://creativecommons.org/licenses/by/4.0)
and is **not** covered by the MIT license; reuse it under its own terms.