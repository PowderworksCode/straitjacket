#!/bin/sh
# Install the straitjacket binary.
#
#   curl -LsSf https://raw.githubusercontent.com/PowderworksCode/straitjacket/main/install.sh | sh
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

# Fetch a URL to stdout, sending credentials only when they were provided.
fetch() {
    if [ -n "${GITHUB_TOKEN:-}" ]; then
        curl -sSfL -H "Authorization: Bearer ${GITHUB_TOKEN}" "$1"
    else
        curl -sSfL "$1"
    fi
}

# The tag of the most recent release.
#
# Parsed out of the API response with sed rather than a JSON library, because
# the whole point of this script is to run before anything is installed.
latest_version() {
    fetch "${API}/releases/latest" |
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

main() {
    need curl
    need tar

    target=$(detect_target)
    version=${STRAITJACKET_VERSION:-}
    if [ -z "$version" ]; then
        version=$(latest_version) ||
            die "could not read the latest release. Set STRAITJACKET_VERSION, or GITHUB_TOKEN if the repository is private"
        [ -n "$version" ] || die "no published release found. Set STRAITJACKET_VERSION to install a specific tag"
    fi

    base=${STRAITJACKET_BASE_URL:-"${DOWNLOAD}/${version}"}
    name="straitjacket-${version}-${target}.tar.gz"
    install_dir=${STRAITJACKET_INSTALL_DIR:-"${HOME}/.local/bin"}

    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT INT TERM

    say "downloading ${name}"
    fetch "${base}/${name}" >"${work}/${name}" ||
        die "could not download ${base}/${name}"
    fetch "${base}/SHA256SUMS" >"${work}/SHA256SUMS" ||
        die "could not download ${base}/SHA256SUMS"
    verify_checksum "${work}/${name}" "${work}/SHA256SUMS" "$name"

    tar -xzf "${work}/${name}" -C "$work"
    [ -f "${work}/straitjacket" ] || die "${name} did not contain a straitjacket binary"

    mkdir -p "$install_dir"
    # Install by rename so a straitjacket that is currently running is replaced
    # rather than written through, which on Linux is a "text file busy" failure.
    mv "${work}/straitjacket" "${install_dir}/straitjacket.new"
    chmod 755 "${install_dir}/straitjacket.new"
    mv "${install_dir}/straitjacket.new" "${install_dir}/straitjacket"

    say "installed ${version} to ${install_dir}/straitjacket"
    case ":${PATH}:" in
        *":${install_dir}:"*) ;;
        *) say "${install_dir} is not on PATH. Add: export PATH=\"${install_dir}:\$PATH\"" ;;
    esac
}

main "$@"
