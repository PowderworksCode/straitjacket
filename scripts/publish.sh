#!/usr/bin/env bash
# Publish straitjacket to crates.io.
#
# Fleet-managed by conf (.ordnung/managed/publishing/rust/publish.sh): edit it
# there. The crate name is substituted from the repository name, so a crate
# named differently from its repository needs a copy of its own.
#
# Publishing is irreversible: a version can be yanked but never deleted, and a
# name/version pair can never be reused. The shape of this script follows from
# that.
#
#   - Dry run is the default. Uploading takes --execute.
#   - An upload of a version the registry already has is skipped rather than
#     attempted, so re-running a release for an existing tag is a no-op instead
#     of a failure. A dry run is never skipped: it packages and compiles every
#     time, because a publish check that returns success without building
#     anything is exactly the green light nobody should trust.
#   - Existing versions are read from the sparse index rather than the web API,
#     because the index is what cargo itself resolves against, and because
#     --index lets the whole path be rehearsed against a local registry.
#   - The version is whatever Cargo.toml says. Nothing here computes or bumps
#     it; the release tag is checked against it in the release workflow.
#
# Usage:
#   scripts/publish.sh                    # dry run: package and verify
#   scripts/publish.sh --execute          # real publish; needs a token
#   scripts/publish.sh --execute --registry local --index ./idx
#
#   --dry-run       package and compile the tarball; never uploads (default)
#   --execute       actually publish
#   --registry NAME publish to a cargo registry other than crates.io
#   --index URL     where to enumerate existing versions; an https:// base or a
#                   local sparse-index directory. Defaults to index.crates.io.
#   --allow-dirty   package with uncommitted changes present
#
# Environment:
#   CARGO_REGISTRY_TOKEN            required for --execute against crates.io.
#   CARGO_REGISTRIES_<NAME>_TOKEN   ... or for --execute --registry <name>.
#   Neither is ever logged.
#
# straitjacket-allow-file:no-comments — this is the procedure for the one
# action in the repository that cannot be undone, and sh has no
# documentation-comment syntax to hoist the reasoning into.
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
CRATE=straitjacket
MODE=dry-run
REGISTRY=""
INDEX_BASE="https://index.crates.io"
ALLOW_DIRTY=0

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run)     MODE=dry-run ;;
    --execute)     MODE=execute ;;
    --registry)    REGISTRY=${2:?--registry needs a name}; shift ;;
    --index)       INDEX_BASE=${2:?--index needs a url or directory}; shift ;;
    --allow-dirty) ALLOW_DIRTY=1 ;;
    # The header is the help text, read rather than duplicated, so the two
    # cannot drift. It stops at the suppression marker below it, which is
    # addressed to the linter rather than to anyone running --help.
    -h|--help)     awk 'NR>1 && /straitjacket-allow-file/ {exit} NR>1 && /^#/ {print; next} NR>1 {exit}' "${BASH_SOURCE[0]}"; exit 0 ;;
    -*)            echo "publish: unknown flag $1" >&2; exit 2 ;;
    *)             echo "publish: unexpected argument $1" >&2; exit 2 ;;
  esac
  shift
done

cd "$ROOT"

VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
[ -n "$VERSION" ] || { echo "publish: no version in Cargo.toml" >&2; exit 1; }

# cargo reads the token for a named registry from CARGO_REGISTRIES_<NAME>_TOKEN,
# and only the default registry from CARGO_REGISTRY_TOKEN.
if [ -n "$REGISTRY" ]; then
  reg_upper=${REGISTRY^^}; reg_upper=${reg_upper//-/_}
  TOKEN_VAR="CARGO_REGISTRIES_${reg_upper}_TOKEN"
else
  TOKEN_VAR=CARGO_REGISTRY_TOKEN
fi
if [ "$MODE" = execute ] && [ -z "${!TOKEN_VAR:-}" ]; then
  cat >&2 <<MSG
publish: --execute needs $TOKEN_VAR and it is not set.

  In CI:   Settings > Secrets and variables > Actions > New repository secret
           Name: CARGO_REGISTRY_TOKEN
           Value: a crates.io API token with publish-new, scoped to $CRATE

  Once the crate exists on crates.io, configure Trusted Publishing instead
  and delete that secret: the release workflow then authenticates over OIDC
  with a token that lives under an hour. See notes/field_guide.md.
MSG
  exit 1
fi

# The sparse index lays a name out as <first two>/<next two>/<name>, and each
# line is one published version.
index_path() {
  local name=$1
  case ${#name} in
    1) printf '1/%s\n' "$name" ;;
    2) printf '2/%s\n' "$name" ;;
    3) printf '3/%s/%s\n' "${name:0:1}" "$name" ;;
    *) printf '%s/%s/%s\n' "${name:0:2}" "${name:2:2}" "$name" ;;
  esac
}

# Whether the registry already carries this version.
#
# A missing entry is the normal case for a first publish and is not an error;
# anything else that goes wrong reads as "not published", and cargo refuses the
# upload on its own if that guess was wrong.
already_published() {
  local path body
  path=$(index_path "$CRATE")
  if [ -d "$INDEX_BASE" ]; then
    [ -f "${INDEX_BASE}/${path}" ] || return 1
    body=$(cat "${INDEX_BASE}/${path}")
  else
    body=$(curl -sSf "${INDEX_BASE}/${path}" 2>/dev/null) || return 1
  fi
  # A herestring rather than a pipe: `grep -q` exits at the first match, and
  # under `set -o pipefail` the SIGPIPE that gives the writer would become the
  # status of the whole pipeline, turning a hit into a miss.
  grep -q "\"vers\"[[:space:]]*:[[:space:]]*\"${VERSION}\"" <<<"$body"
}

# Only --execute is skipped for a version the registry already has: that upload
# can never succeed and re-running a release for an existing tag should be a
# no-op rather than a failure. A dry run still packages and compiles, because a
# publish check that returns success without building anything is exactly the
# green light nobody should trust -- and on every commit that does not bump the
# version, that is every run.
if [ "$MODE" = execute ] && already_published; then
  echo "publish: ${CRATE} ${VERSION} is already on the registry; nothing to upload"
  exit 0
fi

# Written as `if` rather than `test && ...`: under `set -e` a false test as a
# bare statement aborts the script.
args=(publish --locked)
if [ "$MODE" = dry-run ]; then args+=(--dry-run); fi
if [ -n "$REGISTRY" ]; then args+=(--registry "$REGISTRY"); fi
if [ "$ALLOW_DIRTY" = 1 ]; then args+=(--allow-dirty); fi

echo "publish: ${MODE} ${CRATE} ${VERSION}"
cargo "${args[@]}"
