#!/usr/bin/env sh
# ./install.sh
# Purpose: Bootstrap installer for Enchi (formerly pyenv-native) on Linux, macOS, and Termux.
#   Downloads the published release bundle, verifies its SHA-256, and hands over to the bundled
#   `enchi self-install`, which does the actual install (binaries, shell profile, shims, sanity
#   checks) and decides the install root (`~/.enchi`, or an existing `~/.pyenv` it upgrades).
# How to run: curl -fsSL <release>/install.sh | sh
#             sh ./install.sh [--tag vX.Y.Z] [--install-root <dir>] [--shell <name>] [--yes]
# Inputs: Bootstrap options (below) plus any `enchi self-install` option, which is passed through
#   unchanged (--install-root, --shell, --update-shell-profile, --refresh-shims, --log-path, --lang,
#   --yes, --force). Run `enchi self-install --help` for the full list.
#   --github-repo <owner/repo>   Release source (default imyourboyroy/pyenv-native)
#   --tag <vX.Y.Z>               Specific release (default: latest)
#   --release-base-url <url>     Directory containing the bundle and its .sha256
#   --bundle-url <url>           Explicit bundle URL (file:// works); --checksum-url for its sidecar
#   --temp-root <dir>            Scratch directory; --keep-downloads keeps it afterwards
# Environment: ENCHI_INSTALL_GITHUB_REPO, ENCHI_INSTALL_TAG, ENCHI_INSTALL_RELEASE_BASE_URL,
#   ENCHI_INSTALL_BUNDLE_URL, ENCHI_INSTALL_CHECKSUM_URL (the PYENV_NATIVE_INSTALL_* names are still
#   read as a fallback).
# Notes: This script only fetches and verifies. All install logic lives in the Rust binary.

set -eu

# The GitHub repository that publishes releases. It keeps its pre-rename name until the repository
# is renamed on GitHub; that rename is then a one-line edit here.
DEFAULT_GITHUB_REPO="imyourboyroy/pyenv-native"

GITHUB_REPO="${ENCHI_INSTALL_GITHUB_REPO:-${PYENV_NATIVE_INSTALL_GITHUB_REPO:-$DEFAULT_GITHUB_REPO}}"
TAG="${ENCHI_INSTALL_TAG:-${PYENV_NATIVE_INSTALL_TAG:-}}"
RELEASE_BASE_URL="${ENCHI_INSTALL_RELEASE_BASE_URL:-${PYENV_NATIVE_INSTALL_RELEASE_BASE_URL:-}}"
BUNDLE_URL="${ENCHI_INSTALL_BUNDLE_URL:-${PYENV_NATIVE_INSTALL_BUNDLE_URL:-}}"
CHECKSUM_URL="${ENCHI_INSTALL_CHECKSUM_URL:-${PYENV_NATIVE_INSTALL_CHECKSUM_URL:-}}"
TEMP_ROOT="${TMPDIR:-/tmp}/enchi-install"
KEEP_DOWNLOADS="false"
ASSUME_YES="false"

say() { printf '[enchi] %s\n' "$*"; }
fail() { printf 'enchi: %s\n' "$*" >&2; exit 1; }

# Parse bootstrap options; every other argument is forwarded, in order, to `enchi self-install`.
# Unparsed arguments are consumed from the front while forwarded ones are appended behind them.
count=$#
while [ "$count" -gt 0 ]; do
  arg="$1"
  shift
  count=$((count - 1))
  case "$arg" in
    --github-repo|--tag|--release-base-url|--bundle-url|--checksum-url|--temp-root)
      [ "$count" -gt 0 ] || fail "$arg requires a value"
      value="$1"
      shift
      count=$((count - 1))
      case "$arg" in
        --github-repo) GITHUB_REPO="$value" ;;
        --tag) TAG="$value" ;;
        --release-base-url) RELEASE_BASE_URL="$value" ;;
        --bundle-url) BUNDLE_URL="$value" ;;
        --checksum-url) CHECKSUM_URL="$value" ;;
        --temp-root) TEMP_ROOT="$value" ;;
      esac
      ;;
    --keep-downloads) KEEP_DOWNLOADS="true" ;;
    -h|--help)
      sed -n '2,/^set -eu/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      case "$arg" in --yes|-y|--force|-f) ASSUME_YES="true" ;; esac
      set -- "$@" "$arg"
      ;;
  esac
done

is_termux() {
  [ -n "${TERMUX_VERSION:-}" ] && return 0
  case "${PREFIX:-}" in *com.termux*) return 0 ;; esac
  return 1
}

detect_platform() {
  if is_termux; then printf 'android'; return; fi
  case "$(uname -s)" in
    Linux) printf 'linux' ;;
    Darwin) printf 'macos' ;;
    *) fail "unsupported operating system '$(uname -s)'. Use install.ps1 on Windows." ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) printf 'x64' ;;
    arm64|aarch64) printf 'arm64' ;;
    *) fail "no published bundle for architecture '$(uname -m)'" ;;
  esac
}

download() {
  url="$1"; dest="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$dest"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$dest" "$url"
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c 'import sys, urllib.request; urllib.request.urlretrieve(sys.argv[1], sys.argv[2])' "$url" "$dest"
  else
    fail "curl, wget, or python3 is required to download release assets"
  fi
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 "$1" | awk '{print $NF}'
  else
    fail "sha256sum, shasum, or openssl is required to verify the download"
  fi
}

command -v tar >/dev/null 2>&1 || fail "tar is required"

PLATFORM="$(detect_platform)"
ARCH="$(detect_arch)"
ASSET="enchi-${PLATFORM}-${ARCH}.tar.gz"
# Releases cut before the rename only publish this name (same layout, with `pyenv` as the CLI).
LEGACY_ASSET="pyenv-native-${PLATFORM}-${ARCH}.tar.gz"

DERIVED_URL="false"
if [ -z "$BUNDLE_URL" ]; then
  if [ -z "$RELEASE_BASE_URL" ]; then
    [ -n "$GITHUB_REPO" ] || fail "no release source: pass --github-repo, --release-base-url, or --bundle-url"
    if [ -n "$TAG" ]; then
      RELEASE_BASE_URL="https://github.com/${GITHUB_REPO}/releases/download/${TAG}"
    else
      RELEASE_BASE_URL="https://github.com/${GITHUB_REPO}/releases/latest/download"
    fi
  fi
  BUNDLE_URL="${RELEASE_BASE_URL%/}/${ASSET}"
  DERIVED_URL="true"
fi
EXPLICIT_CHECKSUM="false"
[ -z "$CHECKSUM_URL" ] || EXPLICIT_CHECKSUM="true"
[ -n "$CHECKSUM_URL" ] || CHECKSUM_URL="${BUNDLE_URL}.sha256"

mkdir -p "$TEMP_ROOT"
WORK="$(mktemp -d "${TEMP_ROOT%/}/run-XXXXXX")"
cleanup() { [ "$KEEP_DOWNLOADS" = "true" ] || rm -rf "$WORK"; }
trap cleanup EXIT
trap 'exit 130' INT TERM HUP

say "Downloading $BUNDLE_URL"
if ! download "$BUNDLE_URL" "$WORK/$ASSET"; then
  [ "$DERIVED_URL" = "true" ] && [ "$EXPLICIT_CHECKSUM" = "false" ] \
    || fail "could not download $BUNDLE_URL"
  say "That release has no $ASSET; trying the pre-rename asset $LEGACY_ASSET"
  ASSET="$LEGACY_ASSET"
  BUNDLE_URL="${RELEASE_BASE_URL%/}/${ASSET}"
  CHECKSUM_URL="${BUNDLE_URL}.sha256"
  say "Downloading $BUNDLE_URL"
  download "$BUNDLE_URL" "$WORK/$ASSET" || fail "could not download $BUNDLE_URL"
fi
download "$CHECKSUM_URL" "$WORK/$ASSET.sha256"

expected="$(awk 'NR == 1 { print tolower($1) }' "$WORK/$ASSET.sha256")"
printf '%s' "$expected" | grep -Eq '^[0-9a-f]{64}$' || fail "the checksum file did not contain a valid SHA-256 digest"
actual="$(sha256_of "$WORK/$ASSET" | tr '[:upper:]' '[:lower:]')"
[ "$actual" = "$expected" ] || fail "SHA-256 verification failed (expected $expected, got $actual)"
say "Verified SHA-256 for $ASSET"

mkdir -p "$WORK/bundle"
tar -xzf "$WORK/$ASSET" -C "$WORK/bundle"

# The bundle holds `enchi`; bundles from before the rename hold `pyenv` instead.
BUNDLE_DIR="$WORK/bundle"
CLI=""
for name in enchi pyenv; do
  if [ -f "$BUNDLE_DIR/$name" ]; then CLI="$name"; break; fi
done
if [ -z "$CLI" ]; then
  for candidate in "$BUNDLE_DIR"/*/; do
    for name in enchi pyenv; do
      if [ -f "${candidate}${name}" ]; then BUNDLE_DIR="${candidate%/}"; CLI="$name"; break 2; fi
    done
  done
fi
[ -n "$CLI" ] || fail "the downloaded bundle did not contain the enchi binary"
chmod +x "$BUNDLE_DIR/$CLI"

say "Running $CLI self-install"
status=0
# The script's own stdin may be the download pipe, so confirmations read from the terminal.
if [ "$ASSUME_YES" != "true" ] && ( : </dev/tty ) 2>/dev/null; then
  "$BUNDLE_DIR/$CLI" self-install --source-dir "$BUNDLE_DIR" "$@" </dev/tty || status=$?
else
  "$BUNDLE_DIR/$CLI" self-install --source-dir "$BUNDLE_DIR" "$@" || status=$?
fi
exit "$status"
