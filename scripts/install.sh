#!/bin/sh
# Install the straitjacket binary.
#
#   curl -fsSL https://straitjacket.dev/install | sh
#
# Environment:
#   STRAITJACKET_VERSION     version to install, such as v0.1.0. Default: latest
#   STRAITJACKET_INSTALL_DIR where to put the binary. Default: ~/.local/bin
#   STRAITJACKET_BASE_URL    where to fetch release assets from
#   GITHUB_TOKEN             used for the API and download, for a private repository
#
# straitjacket-allow-file:no-comments — this is a script people read before
# piping it to a shell, and sh has no documentation-comment syntax to hoist the
# reasoning into. Explaining itself inline is the whole job.
set -eu

REPO="PowderworksCode/straitjacket"
API="https://api.github.com/repos/${REPO}"
DOWNLOAD="https://github.com/${REPO}/releases/download"

say() {
    printf 'straitjacket: %s\n' "$1" >&2
}

die() {
    printf 'straitjacket: %s\n' "$1" >&2
    exit 1
}

need() {
    command -v "$1" >/dev/null 2>&1 || die "$1 is required and was not found on PATH"
}

# The release target triple for this machine.
#
# The Linux binaries are static musl builds, so one of them runs on any
# distribution regardless of its glibc.
detect_target() {
    kernel=$(uname -s)
    machine=$(uname -m)
    case "$machine" in
        x86_64 | amd64) arch=x86_64 ;;
        aarch64 | arm64) arch=aarch64 ;;
        *) die "unsupported architecture: $machine" ;;
    esac
    case "$kernel" in
        Linux) printf '%s-unknown-linux-musl\n' "$arch" ;;
        Darwin) printf '%s-apple-darwin\n' "$arch" ;;
        *) die "unsupported operating system: $kernel. Build from source with \`cargo install\`" ;;
    esac
}

# Retry a few times, because a reset connection on the way to a release is
# not a reason to fail a build. Rate limiting is not retried away -- see the
# message in `main` -- but a dropped connection usually is.
RETRY="--retry 3 --retry-delay 2"

# Silent when the output is going into a build log, a progress bar when a
# person is watching. A binary is a few megabytes and a slow connection with
# no output at all reads as a hang.
if [ -t 2 ]; then
    PROGRESS="--progress-bar"
else
    PROGRESS="--silent"
fi

# Fetch a URL to stdout, sending credentials only when they were provided.
fetch() {
    # shellcheck disable=SC2086
    if [ -n "${GITHUB_TOKEN:-}" ]; then
        curl -fL --show-error $PROGRESS $RETRY -H "Authorization: Bearer ${GITHUB_TOKEN}" "$1"
    else
        curl -fL --show-error $PROGRESS $RETRY "$1"
    fi
}

# The tag of the most recent release.
#
# Parsed out of the API response with sed rather than a JSON library, because
# the whole point of this script is to run before anything is installed.
#
# The response is captured before it is parsed. Piping the fetch straight into
# sed hides its exit status behind `head`, so a refused request looked exactly
# like a repository with no releases -- and the message said so, which sent
# whoever read it looking in the wrong place.
latest_version() {
    response=$(fetch "${API}/releases/latest") || return 1
    printf '%s\n' "$response" |
        sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' |
        head -n 1
}

# Compare the downloaded archive against the release checksum file.
#
# A truncated download and a tampered one are indistinguishable here, and
# either is a reason to stop rather than to install and hope.
verify_checksum() {
    archive=$1
    sums=$2
    name=$3
    expected=$(sed -n "s/^\([0-9a-f]\{64\}\)[[:space:]]*[*]\{0,1\}${name}\$/\1/p" "$sums" | head -n 1)
    [ -n "$expected" ] || die "${name} is not listed in the release checksums"
    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$archive" | cut -d' ' -f1)
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$archive" | cut -d' ' -f1)
    else
        die "sha256sum or shasum is required to verify the download"
    fi
    [ "$actual" = "$expected" ] || die "checksum mismatch for ${name}: expected ${expected}, got ${actual}"
}

# The API URL of one asset attached to a release.
#
# The API pretty-prints, so `url` and `name` arrive on separate lines and the
# most recent asset `url` is the one the next `name` belongs to. The name is
# compared as a string rather than as a pattern, so the dots in it cannot match
# some other asset. The whole response is read rather than stopped at the match,
# because closing the pipe early makes curl report a write failure that looks
# like a real one.
asset_url() {
    awk -v want="$1" '
        /"url":[[:space:]]*"[^"]*\/releases\/assets\/[0-9]/ {
            line = $0
            sub(/^[^"]*"url":[[:space:]]*"/, "", line)
            sub(/".*$/, "", line)
            url = line
            next
        }
        /"name":[[:space:]]*"/ {
            line = $0
            sub(/^[^"]*"name":[[:space:]]*"/, "", line)
            sub(/".*$/, "", line)
            if (line == want && url != "" && found == "") {
                found = url
            }
        }
        END { if (found != "") print found }
    '
}

# Download one release asset.
#
# A private repository will not serve the plain download URL to a token: it
# redirects to another host and curl drops the credential there, as it should.
# The API asset endpoint is the authenticated path that works, so it is used
# whenever a token was supplied.
#
# The variables are prefixed because sh has no locals, and a plain `name` here
# would overwrite the caller's.
download_asset() {
    asset_name=$1
    asset_output=$2
    if [ -n "${STRAITJACKET_BASE_URL:-}" ]; then
        fetch "${STRAITJACKET_BASE_URL}/${asset_name}" >"$asset_output"
    elif [ -n "${GITHUB_TOKEN:-}" ]; then
        asset_api=$(fetch "${API}/releases/tags/${version}" | asset_url "$asset_name")
        [ -n "$asset_api" ] || die "release ${version} has no asset named ${asset_name}"
        # shellcheck disable=SC2086
        curl -fL --show-error $PROGRESS $RETRY -H "Authorization: Bearer ${GITHUB_TOKEN}" \
            -H "Accept: application/octet-stream" "$asset_api" >"$asset_output"
    else
        # shellcheck disable=SC2086
        curl -fL --show-error $PROGRESS $RETRY "${DOWNLOAD}/${version}/${asset_name}" >"$asset_output"
    fi
}

main() {
    need curl
    need tar
    need awk

    target=$(detect_target)
    version=${STRAITJACKET_VERSION:-}
    if [ -z "$version" ]; then
        version=$(latest_version) || die "could not read the latest release.
  GitHub allows 60 unauthenticated API requests an hour per address, and CI
  runners share addresses, so this is usually rate limiting rather than a
  missing release. Set GITHUB_TOKEN -- any valid token raises the limit, and
  it need not have access to this repository while it is public. Or pin
  STRAITJACKET_VERSION, which skips the API entirely."
        [ -n "$version" ] || die "no published release found. Set STRAITJACKET_VERSION to install a specific tag"
    fi

    # Releases are tagged vX.Y.Z. Accepting X.Y.Z as well means the obvious
    # value for STRAITJACKET_VERSION does the obvious thing, rather than asking
    # for an asset that was never published under that name.
    case "$version" in
        v*) ;;
        *) version="v${version}" ;;
    esac

    name="straitjacket-${version}-${target}.tar.gz"
    install_dir=${STRAITJACKET_INSTALL_DIR:-"${HOME}/.local/bin"}

    work=$(mktemp -d)
    # The half-installed binary goes too. Leaving straitjacket.new behind means
    # the next run finds a file it did not write and cannot reason about.
    trap 'rm -rf "$work"; rm -f "${install_dir}/straitjacket.new"' EXIT INT TERM

    say "downloading ${name}"
    download_asset "$name" "${work}/${name}" ||
        die "could not download ${name} from release ${version}"
    download_asset SHA256SUMS "${work}/SHA256SUMS" ||
        die "could not download SHA256SUMS from release ${version}"
    verify_checksum "${work}/${name}" "${work}/SHA256SUMS" "$name"

    tar -xzf "${work}/${name}" -C "$work"
    [ -f "${work}/straitjacket" ] || die "${name} did not contain a straitjacket binary"

    mkdir -p "$install_dir"
    # Install by rename so a straitjacket that is currently running is replaced
    # rather than written through, which on Linux is a "text file busy" failure.
    mv "${work}/straitjacket" "${install_dir}/straitjacket.new"
    chmod 755 "${install_dir}/straitjacket.new"
    mv "${install_dir}/straitjacket.new" "${install_dir}/straitjacket"

    # Run it once before claiming it is installed. A binary for the wrong
    # architecture downloads and unpacks perfectly and fails at the first use,
    # which is a long way from here if that use is in someone's CI.
    installed=$("${install_dir}/straitjacket" --version 2>/dev/null) || die \
"installed ${install_dir}/straitjacket, but it would not run.
  The archive was built for ${target}, which is what this machine reports.
  If that is wrong, install with \`cargo install straitjacket\` instead."

    say "installed ${installed} to ${install_dir}/straitjacket"
    case ":${PATH}:" in
        *":${install_dir}:"*) ;;
        *) say "${install_dir} is not on PATH. Add: export PATH=\"${install_dir}:\$PATH\"" ;;
    esac
}

main "$@"
