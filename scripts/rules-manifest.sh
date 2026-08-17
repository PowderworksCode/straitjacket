#!/usr/bin/env bash
# Regenerate site/content/rules.json from the binary.
#
# The manifest is what the documentation is checked against, so it has to come
# out of the scanner rather than be maintained beside it. `--no-config` keeps
# the checked-in repository configuration from leaking into the exported
# defaults, which must be the built-in ones the docs describe.
#
# straitjacket-allow-file:no-comments — a script is read before it is run, and
# sh has no documentation-comment syntax to hoist the reasoning into.
set -euo pipefail

cd "$(dirname "$0")/.."

OUT=site/content/rules.json
CHECK=0
[[ ${1:-} == "--check" ]] && CHECK=1

cargo build --release --quiet
./target/release/straitjacket --list-rules --no-config --format json >"${OUT}.new"

if [[ $CHECK -eq 1 ]]; then
    if ! diff -u "$OUT" "${OUT}.new"; then
        rm -f "${OUT}.new"
        echo "rules-manifest: $OUT is stale. Run scripts/rules-manifest.sh and commit the result." >&2
        exit 1
    fi
    rm -f "${OUT}.new"
    echo "rules-manifest: $OUT matches the binary"
else
    mv "${OUT}.new" "$OUT"
    echo "rules-manifest: wrote $OUT"
fi
