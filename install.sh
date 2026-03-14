#!/usr/bin/env sh
# ./install.sh
# Purpose: Downloads a published pyenv-native POSIX bundle, verifies it, and runs the bundled installer without requiring a repo clone.
# How to run: sh ./install.sh [--github-repo <owner/repo>] [--tag <vX.Y.Z>] [--install-root <dir>]
# Inputs: Optional GitHub repo/tag or direct release URLs, install root, shell/profile toggles, temp cache location, and overwrite/cleanup flags.
# Outputs/side effects: Downloads the Linux/macOS release bundle plus checksum, verifies SHA-256, extracts it into a temp directory, and installs pyenv-native into the requested portable root.
# Notes: Designed for copy-paste web installs and keeps installs portable under a pyenv-managed root.

set -eu

GITHUB_REPO="${PYENV_NATIVE_INSTALL_GITHUB_REPO:-imyourboyroy/pyenv-native}"
TAG="${PYENV_NATIVE_INSTALL_TAG:-}"
RELEASE_BASE_URL="${PYENV_NATIVE_INSTALL_RELEASE_BASE_URL:-}"
BUNDLE_URL="${PYENV_NATIVE_INSTALL_BUNDLE_URL:-}"
CHECKSUM_URL="${PYENV_NATIVE_INSTALL_CHECKSUM_URL:-}"
INSTALL_ROOT="${HOME}/.pyenv"
SHELL_KIND=""
ADD_TO_USER_PATH="true"
UPDATE_SHELL_PROFILE="true"
REFRESH_SHIMS="true"
TEMP_ROOT="${TMPDIR:-/tmp}/pyenv-native-install"
KEEP_DOWNLOADS="false"
FORCE="false"

write_step() {
  printf '[pyenv-native] %s\n' "$1"
}

parse_bool() {
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|on) printf '%s\n' "true" ;;
    0|false|no|off) printf '%s\n' "false" ;;
    *) printf 'Invalid boolean value `%s`\n' "$1" >&2; exit 1 ;;
  esac
}

detect_shell_kind() {
  shell_name="$(basename -- "${SHELL:-sh}" | tr '[:upper:]' '[:lower:]')"
  case "$shell_name" in
    bash|zsh|fish|sh) printf '%s\n' "$shell_name" ;;
    *) printf '%s\n' "sh" ;;
  esac
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
    *) uname -m | tr '[:upper:]' '[:lower:]' ;;
  esac
}

bundle_file_name() {
  operating_system="$1"
  architecture="$2"
  printf 'pyenv-native-%s-%s.tar.gz\n' "$operating_system" "$architecture"
}

download_file() {
  url="$1"
  destination_path="$2"
  mkdir -p "$(dirname -- "$destination_path")"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$destination_path"
    return
  fi
  if command -v wget >/dev/null 2>&1; then
    wget -qO "$destination_path" "$url"
    return
  fi
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$url" "$destination_path" <<'PY'
import pathlib
import sys
import urllib.request

url = sys.argv[1]
destination = pathlib.Path(sys.argv[2])
destination.parent.mkdir(parents=True, exist_ok=True)
with urllib.request.urlopen(url) as response, destination.open("wb") as handle:
    handle.write(response.read())
PY
    return
  fi

  printf '%s\n' "Unable to download release assets: curl, wget, or python3 is required." >&2
  exit 1
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

read_expected_checksum() {
  checksum_path="$1"
  if [ ! -f "$checksum_path" ]; then
    printf 'Checksum file `%s` was not found.\n' "$checksum_path" >&2
    exit 1
  fi
  expected="$(awk 'NR == 1 { print $1 }' "$checksum_path" | tr '[:upper:]' '[:lower:]')"
  case "$expected" in
    [0-9a-f]*)
      if [ "${#expected}" -eq 64 ]; then
        printf '%s\n' "$expected"
        return
      fi
      ;;
  esac
  printf 'Checksum file `%s` did not contain a valid SHA-256 digest.\n' "$checksum_path" >&2
  exit 1
}

resolve_release_urls() {
  operating_system="$1"
  architecture="$2"
  asset_name="$(bundle_file_name "$operating_system" "$architecture")"

  if [ -n "$BUNDLE_URL" ]; then
    resolved_checksum_url="$CHECKSUM_URL"
    if [ -z "$resolved_checksum_url" ]; then
      resolved_checksum_url="${BUNDLE_URL}.sha256"
    fi
    printf '%s\n%s\n%s\n%s\n' "$BUNDLE_URL" "$resolved_checksum_url" "$asset_name" "explicit bundle url"
    return
  fi

  resolved_base_url="$RELEASE_BASE_URL"
  source_label=""
  if [ -z "$resolved_base_url" ]; then
    if [ -z "$GITHUB_REPO" ]; then
      printf '%s\n' "Unable to resolve a release source. Pass --github-repo, --release-base-url, or --bundle-url." >&2
      exit 1
    fi
    if [ -n "$TAG" ]; then
      resolved_base_url="https://github.com/${GITHUB_REPO}/releases/download/${TAG}"
      source_label="github release ${GITHUB_REPO}@${TAG}"
    else
      resolved_base_url="https://github.com/${GITHUB_REPO}/releases/latest/download"
      source_label="latest github release for ${GITHUB_REPO}"
    fi
  else
    source_label="release base url ${resolved_base_url}"
  fi

  bundle_url="${resolved_base_url%/}/${asset_name}"
  resolved_checksum_url="$CHECKSUM_URL"
  if [ -z "$resolved_checksum_url" ]; then
    resolved_checksum_url="${bundle_url}.sha256"
  fi
  printf '%s\n%s\n%s\n%s\n' "$bundle_url" "$resolved_checksum_url" "$asset_name" "$source_label"
}

assert_install_root_state() {
  resolved_install_root="$1"
  installed_executable="${resolved_install_root}/bin/pyenv"
  if [ -e "$installed_executable" ]; then
    if [ "$FORCE" != "true" ]; then
      printf 'pyenv-native is already installed at %s. Re-run with --force to upgrade in place or run uninstall.sh first.\n' "$installed_executable" >&2
      exit 1
    fi
    write_step "Existing pyenv-native install detected at ${installed_executable}; continuing because --force was supplied."
    return
  fi

  if [ -d "$resolved_install_root" ] && [ -n "$(find "$resolved_install_root" -mindepth 1 -maxdepth 1 2>/dev/null | head -n 1)" ] && [ "$FORCE" != "true" ]; then
    printf 'Install root `%s` already exists and is not empty. Re-run with --force or choose a different --install-root.\n' "$resolved_install_root" >&2
    exit 1
  fi
}

warn_existing_path_command() {
  resolved_install_root="$1"
  if ! command -v pyenv >/dev/null 2>&1; then
    return
  fi

  existing_command="$(command -v pyenv)"
  expected_prefix="${resolved_install_root}/bin/"
  case "$existing_command" in
    "${expected_prefix}"*) ;;
    *)
      printf 'Warning: a different pyenv command is already discoverable at `%s`. Restart shells after install and verify PATH ordering.\n' "$existing_command" >&2
      ;;
  esac
}

cleanup_paths() {
  for cleanup_path in "$@"; do
    if [ -n "$cleanup_path" ] && [ -e "$cleanup_path" ]; then
      rm -rf "$cleanup_path"
    fi
  done
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --github-repo)
      GITHUB_REPO="${2:-}"
      shift 2
      ;;
    --tag)
      TAG="${2:-}"
      shift 2
      ;;
    --release-base-url)
      RELEASE_BASE_URL="${2:-}"
      shift 2
      ;;
    --bundle-url)
      BUNDLE_URL="${2:-}"
      shift 2
      ;;
    --checksum-url)
      CHECKSUM_URL="${2:-}"
      shift 2
      ;;
    --install-root)
      INSTALL_ROOT="${2:-}"
      shift 2
      ;;
    --shell)
      SHELL_KIND="${2:-}"
      shift 2
      ;;
    --add-to-user-path)
      ADD_TO_USER_PATH="$(parse_bool "${2:-}")"
      shift 2
      ;;
    --update-shell-profile)
      UPDATE_SHELL_PROFILE="$(parse_bool "${2:-}")"
      shift 2
      ;;
    --refresh-shims)
      REFRESH_SHIMS="$(parse_bool "${2:-}")"
      shift 2
      ;;
    --temp-root)
      TEMP_ROOT="${2:-}"
      shift 2
      ;;
    --keep-downloads)
      KEEP_DOWNLOADS="true"
      shift 1
      ;;
    --force)
      FORCE="true"
      shift 1
      ;;
    *)
      printf 'Unknown option `%s`\n' "$1" >&2
      exit 1
      ;;
  esac
done

OPERATING_SYSTEM="$(normalize_os)"
if [ "$OPERATING_SYSTEM" = "unsupported" ]; then
  printf '%s\n' "install.sh currently supports Linux and macOS hosts only. Use install.ps1 on Windows." >&2
  exit 1
fi

ARCHITECTURE="$(normalize_arch)"
case "$ARCHITECTURE" in
  x64|arm64) ;;
  *)
    printf 'Published %s bundles are not available yet for architecture `%s`.\n' "$OPERATING_SYSTEM" "$ARCHITECTURE" >&2
    exit 1
    ;;
esac

if [ -z "$SHELL_KIND" ]; then
  SHELL_KIND="$(detect_shell_kind)"
fi

if ! command -v tar >/dev/null 2>&1; then
  printf '%s\n' "tar is required to install pyenv-native on POSIX hosts." >&2
  exit 1
fi

ASSET_INFO="$(resolve_release_urls "$OPERATING_SYSTEM" "$ARCHITECTURE")"
BUNDLE_URL="$(printf '%s' "$ASSET_INFO" | sed -n '1p')"
CHECKSUM_URL="$(printf '%s' "$ASSET_INFO" | sed -n '2p')"
ASSET_NAME="$(printf '%s' "$ASSET_INFO" | sed -n '3p')"
SOURCE_LABEL="$(printf '%s' "$ASSET_INFO" | sed -n '4p')"

mkdir -p "$TEMP_ROOT"
TEMP_ROOT="$(CDPATH= cd -- "$TEMP_ROOT" && pwd)"
mkdir -p "$(dirname -- "$INSTALL_ROOT")"
INSTALL_ROOT="$(CDPATH= cd -- "$(dirname -- "$INSTALL_ROOT")" && pwd)/$(basename -- "$INSTALL_ROOT")"
DOWNLOAD_ROOT="$(mktemp -d "${TEMP_ROOT}/downloads-XXXXXX")"
EXTRACT_ROOT="$(mktemp -d "${TEMP_ROOT}/extract-XXXXXX")"

if [ "$KEEP_DOWNLOADS" != "true" ]; then
  trap 'cleanup_paths "$DOWNLOAD_ROOT" "$EXTRACT_ROOT"' EXIT HUP INT TERM
fi

write_step "Preparing install for ${INSTALL_ROOT}"
warn_existing_path_command "$INSTALL_ROOT"
assert_install_root_state "$INSTALL_ROOT"

BUNDLE_PATH="${DOWNLOAD_ROOT}/${ASSET_NAME}"
CHECKSUM_PATH="${BUNDLE_PATH}.sha256"

write_step "Downloading ${SOURCE_LABEL}"
download_file "$BUNDLE_URL" "$BUNDLE_PATH"
download_file "$CHECKSUM_URL" "$CHECKSUM_PATH"

EXPECTED_SHA="$(read_expected_checksum "$CHECKSUM_PATH")"
ACTUAL_SHA="$(sha256_for_file "$BUNDLE_PATH" | tr '[:upper:]' '[:lower:]')"
if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
  printf 'SHA-256 verification failed for `%s`. Expected %s but found %s.\n' "$BUNDLE_PATH" "$EXPECTED_SHA" "$ACTUAL_SHA" >&2
  exit 1
fi
write_step "Verified SHA-256 for ${ASSET_NAME}"

tar -xzf "$BUNDLE_PATH" -C "$EXTRACT_ROOT"
INSTALLER_PATH="${EXTRACT_ROOT}/install-pyenv-native.sh"
EXECUTABLE_PATH="${EXTRACT_ROOT}/pyenv"
MANIFEST_PATH="${EXTRACT_ROOT}/bundle-manifest.json"

for required_path in "$INSTALLER_PATH" "$EXECUTABLE_PATH" "$MANIFEST_PATH"; do
  if [ ! -f "$required_path" ]; then
    printf 'Downloaded bundle was missing required file `%s`.\n' "$required_path" >&2
    exit 1
  fi
done

chmod +x "$INSTALLER_PATH" "$EXECUTABLE_PATH"
if ! grep -q '"platform"[[:space:]]*:[[:space:]]*"'$OPERATING_SYSTEM'"' "$MANIFEST_PATH"; then
  printf 'Downloaded bundle manifest did not match host platform `%s`.\n' "$OPERATING_SYSTEM" >&2
  exit 1
fi

write_step "Running bundled installer from ${INSTALLER_PATH}"
if [ "$FORCE" = "true" ]; then
  "$INSTALLER_PATH" \
    --source-path "$EXECUTABLE_PATH" \
    --install-root "$INSTALL_ROOT" \
    --shell "$SHELL_KIND" \
    --add-to-user-path "$ADD_TO_USER_PATH" \
    --update-shell-profile "$UPDATE_SHELL_PROFILE" \
    --refresh-shims "$REFRESH_SHIMS" \
    --force
else
  "$INSTALLER_PATH" \
    --source-path "$EXECUTABLE_PATH" \
    --install-root "$INSTALL_ROOT" \
    --shell "$SHELL_KIND" \
    --add-to-user-path "$ADD_TO_USER_PATH" \
    --update-shell-profile "$UPDATE_SHELL_PROFILE" \
    --refresh-shims "$REFRESH_SHIMS"
fi

printf '\nInstalled pyenv-native to %s\n' "$INSTALL_ROOT"
printf 'Bundle source: %s\n' "$SOURCE_LABEL"
printf 'Installed command: %s\n' "${INSTALL_ROOT}/bin/pyenv"
if [ -n "$GITHUB_REPO" ]; then
  printf 'Remote uninstall helper: https://raw.githubusercontent.com/%s/main/uninstall.sh\n' "$GITHUB_REPO"
fi
