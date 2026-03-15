#!/usr/bin/env sh
# ./scripts/build-release-bundle.sh
# Purpose: Builds release binaries and assembles a portable POSIX distribution bundle for pyenv-native.
# How to run: sh ./scripts/build-release-bundle.sh [--output-root ./dist] [--bundle-name pyenv-native-linux-x64]
# Inputs: Optional output root and bundle name override; otherwise infers the host OS/architecture for asset naming.
# Outputs/side effects: Builds the release binaries, writes a bundle directory under dist/, and creates a .tar.gz archive with installers, the MCP server, and user-facing docs.
# Notes: Intended for Linux/macOS packaging; uses tar.gz to preserve executable permissions for the bundled binaries and shell scripts.

set -eu

resolve_script_dir() {
  CDPATH= cd -- "$(dirname -- "$0")" && pwd
}

normalize_os() {
  case "$(uname -s | tr '[:upper:]' '[:lower:]')" in
    linux*) printf '%s\n' "linux" ;;
    darwin*) printf '%s\n' "macos" ;;
    *) printf '%s\n' "unsupported" ;;
  esac
}

normalize_arch() {
  case "$(uname -m | tr '[:upper:]' '[:lower:]')" in
    x86_64|amd64|x64) printf '%s\n' "x64" ;;
    arm64|aarch64) printf '%s\n' "arm64" ;;
    x86|i386|i686) printf '%s\n' "x86" ;;
    *) uname -m | tr '[:upper:]' '[:lower:]' ;;
  esac
}

sha256_for_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
    return
  fi
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$1" <<'PY'
import hashlib
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
digest = hashlib.sha256()
with path.open("rb") as handle:
    for chunk in iter(lambda: handle.read(1024 * 1024), b""):
        digest.update(chunk)
print(digest.hexdigest())
PY
    return
  fi

  printf '%s\n' "Unable to calculate SHA-256: sha256sum, shasum, or python3 is required." >&2
  exit 1
}

SCRIPT_DIR="$(resolve_script_dir)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_ROOT="${REPO_ROOT}/dist"
BUNDLE_NAME=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-root)
      OUTPUT_ROOT="${2:-}"
      shift 2
      ;;
    --bundle-name)
      BUNDLE_NAME="${2:-}"
      shift 2
      ;;
    *)
      printf 'Unknown option `%s`\n' "$1" >&2
      exit 1
      ;;
  esac
done

OPERATING_SYSTEM="$(normalize_os)"
ARCHITECTURE="$(normalize_arch)"
if [ "$OPERATING_SYSTEM" = "unsupported" ]; then
  printf 'Unsupported host operating system for POSIX bundle production.\n' >&2
  exit 1
fi

if [ -z "$BUNDLE_NAME" ]; then
  BUNDLE_NAME="pyenv-native-${OPERATING_SYSTEM}-${ARCHITECTURE}"
fi

OUTPUT_ROOT="$(mkdir -p "$OUTPUT_ROOT" && CDPATH= cd -- "$OUTPUT_ROOT" && pwd)"
BUNDLE_DIR="${OUTPUT_ROOT}/${BUNDLE_NAME}"
ARCHIVE_PATH="${OUTPUT_ROOT}/${BUNDLE_NAME}.tar.gz"
CHECKSUM_PATH="${ARCHIVE_PATH}.sha256"
RELEASE_BIN="${REPO_ROOT}/target/release/pyenv"
RELEASE_MCP_BIN="${REPO_ROOT}/target/release/pyenv-mcp"
CARGO_TOML_PATH="${REPO_ROOT}/Cargo.toml"

cargo build --release --bin pyenv --bin pyenv-mcp

for required_binary in "$RELEASE_BIN" "$RELEASE_MCP_BIN"; do
  if [ ! -f "$required_binary" ]; then
    printf 'Release binary was not found at %s\n' "$required_binary" >&2
    exit 1
  fi
done

if [ -d "$BUNDLE_DIR" ]; then
  rm -rf "$BUNDLE_DIR"
fi
mkdir -p "$BUNDLE_DIR"

BUNDLE_VERSION="$(sed -n 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"\([^"]*\)"[[:space:]]*$/\1/p' "$CARGO_TOML_PATH" | head -n 1)"
if [ -z "$BUNDLE_VERSION" ]; then
  printf 'Failed to determine workspace version from %s\n' "$CARGO_TOML_PATH" >&2
  exit 1
fi

cp "$RELEASE_BIN" "${BUNDLE_DIR}/pyenv"
cp "$RELEASE_MCP_BIN" "${BUNDLE_DIR}/pyenv-mcp"
cp "${REPO_ROOT}/README.md" "${BUNDLE_DIR}/README.md"
cp "${REPO_ROOT}/INSTRUCTIONS.md" "${BUNDLE_DIR}/INSTRUCTIONS.md"
if [ -f "${REPO_ROOT}/MCP.md" ]; then
  cp "${REPO_ROOT}/MCP.md" "${BUNDLE_DIR}/MCP.md"
fi
cp "${REPO_ROOT}/LICENSE" "${BUNDLE_DIR}/LICENSE"
cp "${SCRIPT_DIR}/install-pyenv-native.sh" "${BUNDLE_DIR}/install-pyenv-native.sh"
cp "${SCRIPT_DIR}/uninstall-pyenv-native.sh" "${BUNDLE_DIR}/uninstall-pyenv-native.sh"
chmod +x "${BUNDLE_DIR}/pyenv" "${BUNDLE_DIR}/pyenv-mcp" "${BUNDLE_DIR}/install-pyenv-native.sh" "${BUNDLE_DIR}/uninstall-pyenv-native.sh"

cat > "${BUNDLE_DIR}/bundle-manifest.json" <<EOF
{
  "bundle_name": "${BUNDLE_NAME}",
  "bundle_version": "${BUNDLE_VERSION}",
  "platform": "${OPERATING_SYSTEM}",
  "architecture": "${ARCHITECTURE}",
  "executable": "pyenv",
  "mcp_executable": "pyenv-mcp",
  "install_script": "install-pyenv-native.sh",
  "uninstall_script": "uninstall-pyenv-native.sh"
}
EOF

if [ -f "$ARCHIVE_PATH" ]; then
  rm -f "$ARCHIVE_PATH"
fi
if [ -f "$CHECKSUM_PATH" ]; then
  rm -f "$CHECKSUM_PATH"
fi

COPYFILE_DISABLE=1 tar -C "$BUNDLE_DIR" -czf "$ARCHIVE_PATH" .
ARCHIVE_HASH="$(sha256_for_file "$ARCHIVE_PATH")"
printf '%s  %s\n' "$ARCHIVE_HASH" "$(basename "$ARCHIVE_PATH")" > "$CHECKSUM_PATH"

printf 'repo_root: %s\n' "$REPO_ROOT"
printf 'bundle_dir: %s\n' "$BUNDLE_DIR"
printf 'archive_path: %s\n' "$ARCHIVE_PATH"
printf 'checksum_path: %s\n' "$CHECKSUM_PATH"
printf 'release_bin: %s\n' "$RELEASE_BIN"
printf 'release_mcp_bin: %s\n' "$RELEASE_MCP_BIN"
