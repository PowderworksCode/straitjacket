#!/usr/bin/env bash
# Install and build what working on Straitjacket needs: the Rust workspace it
# ships, and the site's dependencies so the documentation can be built and its
# tests run.
#
# straitjacket-allow-file:no-comments — a setup script is read before it is
# run, and sh has no documentation-comment syntax to hoist the reasoning into.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> building the Rust workspace"
cargo build --workspace

echo "==> installing site dependencies"
(
    cd site
    bun install --frozen-lockfile
)

# Git looks in .git/hooks by default, which nothing tracks. The fleet's hooks
# are committed under .githooks, so they only run once a checkout is pointed at
# them; doing it here means a clone that ran this script is a clone that has
# them.
echo "==> pointing Git at the committed hooks"
git config core.hooksPath .githooks

echo "Development environment ready. Run scripts/rules-manifest.sh after changing a rule."
