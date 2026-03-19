#!/usr/bin/env sh
# ./install.sh
# Purpose: Downloads a published pyenv-native POSIX bundle, verifies it, and runs the bundled installer without requiring a repo clone.
# How to run: sh ./install.sh [--github-repo <owner/repo>] [--tag <vX.Y.Z>] [--install-root <dir>] [--yes]
# Inputs: Optional GitHub repo/tag or direct release URLs, install root, shell/profile toggles, temp cache location, logging location, and overwrite/cleanup flags.
# Outputs/side effects: Downloads the Linux/macOS release bundle plus checksum, verifies SHA-256, extracts it into a temp directory, and installs pyenv-native into the requested portable root.
# Notes: Designed for copy-paste web installs, defaults to the latest published GitHub release, and keeps installs portable under a pyenv-managed root.

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
YES="false"
LOG_PATH=""

print_line() {
  printf '%s\n' "$1"
}

parse_bool() {
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|on) print_line "true" ;;
    0|false|no|off) print_line "false" ;;
    *)
      printf 'Invalid boolean value `%s`. Use true/false, yes/no, on/off, or 1/0.\n' "$1" >&2
      exit 1
      ;;
  esac
}

detect_shell_kind() {
  if [ -z "${SHELL:-}" ] && is_termux; then
    print_line "bash"
    return
  fi
  shell_name="$(basename -- "${SHELL:-sh}" | tr '[:upper:]' '[:lower:]')"
  case "$shell_name" in
    bash|zsh|fish|sh) print_line "$shell_name" ;;
    *) print_line "sh" ;;
  esac
}

is_termux() {
  [ -n "${TERMUX_VERSION:-}" ] && return 0
  case "${PREFIX:-}" in
    *com.termux*|*/data/data/com.termux/*) return 0 ;;
  esac
  return 1
}

normalize_os() {
  case "$(uname -s | tr '[:upper:]' '[:lower:]')" in
    linux*) print_line linux ;;
    darwin*) print_line macos ;;
    *) print_line unsupported ;;
  esac
}

normalize_arch() {
  case "$(uname -m | tr '[:upper:]' '[:lower:]')" in
    x86_64|amd64|x64) print_line x64 ;;
    arm64|aarch64) print_line arm64 ;;
    *) uname -m | tr '[:upper:]' '[:lower:]' ;;
  esac
}

bundle_file_name() {
  print_line "pyenv-native-$1-$2.tar.gz"
}

is_interactive() {
  [ -r /dev/tty ]
}

nearest_existing_dir() {
  candidate="$1"
  while [ ! -e "$candidate" ]; do
    parent="$(dirname -- "$candidate")"
    if [ "$parent" = "$candidate" ]; then
      break
    fi
    candidate="$parent"
  done

  if [ -f "$candidate" ]; then
    dirname -- "$candidate"
  else
    print_line "$candidate"
  fi
}

test_directory_writable() {
  target_dir="$1"
  probe_path="$target_dir/.pyenv-native-write-test-$$"
  if : > "$probe_path" 2>/dev/null; then
    rm -f "$probe_path"
    return 0
  fi
  return 1
}

assert_install_root_access() {
  anchor="$(nearest_existing_dir "$(dirname -- "$INSTALL_ROOT")")"
  if test_directory_writable "$anchor"; then
    return 0
  fi

  if [ "$(id -u)" -eq 0 ]; then
    printf 'Install root `%s` is not writable even as root. Choose a different --install-root.\n' "$INSTALL_ROOT" >&2
  else
    printf 'Install root `%s` requires elevated permissions. Re-run with sudo or choose a user-writable --install-root.\n' "$INSTALL_ROOT" >&2
  fi
  exit 1
}

assert_install_root_state() {
  installed_executable="${INSTALL_ROOT}/bin/pyenv"
  if [ -e "$installed_executable" ] && [ "$FORCE" != "true" ]; then
    printf 'Warning: pyenv-native is already installed at %s. Proceeding will upgrade or overwrite the installation in-place.\n' "$installed_executable" >&2
  fi

  if [ -d "$INSTALL_ROOT" ] \
    && [ -n "$(find "$INSTALL_ROOT" -mindepth 1 -maxdepth 1 2>/dev/null | head -n 1)" ] \
    && [ ! -e "$installed_executable" ] \
    && [ "$FORCE" != "true" ]; then
    printf 'Warning: Install root `%s` already exists and is not empty. Proceeding will install into this existing directory.\n' "$INSTALL_ROOT" >&2
  fi
}

warn_existing_path_command() {
  if ! command -v pyenv >/dev/null 2>&1; then
    return
  fi

  existing_command="$(command -v pyenv)"
  expected_prefix="${INSTALL_ROOT}/bin/"
  case "$existing_command" in
    "${expected_prefix}"*) ;;
    *)
      printf 'Warning: a different pyenv command is already discoverable at `%s`. Restart your shell after install and verify PATH ordering.\n' "$existing_command" >&2
      ;;
  esac
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
with urllib.request.urlopen(url) as response, destination.open('wb') as handle:
    handle.write(response.read())
PY
    return
  fi

  print_line 'Unable to download release assets: curl, wget, or python3 is required.' >&2
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
with path.open('rb') as handle:
    for chunk in iter(lambda: handle.read(1024 * 1024), b''):
        digest.update(chunk)
print(digest.hexdigest())
PY
    return
  fi

  print_line 'Unable to calculate SHA-256: sha256sum, shasum, or python3 is required.' >&2
  exit 1
}

read_expected_checksum() {
  checksum_path="$1"
  expected="$(awk 'NR == 1 { print $1 }' "$checksum_path" | tr '[:upper:]' '[:lower:]')"
  case "$expected" in
    [0-9a-f]*)
      if [ "${#expected}" -eq 64 ]; then
        print_line "$expected"
        return
      fi
      ;;
  esac
  printf 'Checksum file `%s` did not contain a valid SHA-256 digest.\n' "$checksum_path" >&2
  exit 1
}

resolve_release_urls() {
  asset_name="$(bundle_file_name "$1" "$2")"

  if [ -n "$BUNDLE_URL" ]; then
    resolved_checksum_url="$CHECKSUM_URL"
    if [ -z "$resolved_checksum_url" ]; then
      resolved_checksum_url="${BUNDLE_URL}.sha256"
    fi
    printf '%s\n%s\n%s\n%s\n' "$BUNDLE_URL" "$resolved_checksum_url" "$asset_name" 'explicit bundle url'
    return
  fi

  resolved_base_url="$RELEASE_BASE_URL"
  source_label=''
  if [ -z "$resolved_base_url" ]; then
    if [ -z "$GITHUB_REPO" ]; then
      print_line 'Unable to resolve a release source. Pass --github-repo, --release-base-url, or --bundle-url.' >&2
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

ensure_log_ready() {
  if [ -z "$LOG_PATH" ]; then
    timestamp="$(date +%Y%m%d-%H%M%S 2>/dev/null || print_line unknown)"
    LOG_PATH="${INSTALL_ROOT}/logs/network-install-${timestamp}.log"
  fi
  mkdir -p "$(dirname -- "$LOG_PATH")"
  : > "$LOG_PATH"
}

log_line() {
  level="$1"
  message="$2"
  line="[pyenv-native][$level] $message"
  print_line "$line"
  if [ -n "$LOG_PATH" ]; then
    print_line "$line" >> "$LOG_PATH"
  fi
}

write_step() {
  log_line INFO "$1"
}

cleanup_paths() {
  for cleanup_path in "$@"; do
    if [ -n "$cleanup_path" ] && [ -e "$cleanup_path" ]; then
      rm -rf "$cleanup_path"
    fi
  done
}

emit_summary() {
  print_line ""
  print_line "pyenv-native network install summary"
  print_line "===================================="
  print_line "release source : $SOURCE_LABEL"
  print_line "bundle url     : $BUNDLE_URL"
  print_line "checksum url   : $CHECKSUM_URL"
  print_line "install root   : $INSTALL_ROOT"
  print_line "shell          : $SHELL_KIND"
  print_line "profile update : $UPDATE_PROFILE_EFFECTIVE"
  print_line "path hint      : $ADD_TO_USER_PATH"
  print_line "refresh shims  : $REFRESH_SHIMS"
  print_line "temp root      : $TEMP_ROOT"
  print_line "force          : $FORCE"
  print_line "log path       : $LOG_PATH"
  print_line ""
  print_line "This will download a published pyenv-native bundle, verify its SHA-256 checksum, and install it into the selected portable root."
  if [ "$UPDATE_PROFILE_EFFECTIVE" = "true" ]; then
    print_line "Your shell profile will be updated so future sessions can find pyenv-native automatically."
  else
    print_line "No shell profile changes will be made."
  fi
  print_line ""
}

confirm_action() {
  if [ "$YES" = "true" ] || [ "$FORCE" = "true" ]; then
    return 0
  fi

  if ! is_interactive; then
    print_line 'Confirmation is required for interactive installs. Re-run with --yes for non-interactive use.' >&2
    exit 1
  fi

  printf 'Continue with install? [y/N]: ' > /dev/tty
  if ! IFS= read -r answer < /dev/tty; then
    print_line 'Install cancelled.' >&2
    exit 1
  fi

  case "$(printf '%s' "$answer" | tr '[:upper:]' '[:lower:]')" in
    y|yes) return 0 ;;
    *)
      print_line 'Install cancelled.' >&2
      exit 1
      ;;
  esac
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
    --log-path)
      LOG_PATH="${2:-}"
      shift 2
      ;;
    --keep-downloads)
      KEEP_DOWNLOADS=true
      shift 1
      ;;
    --force)
      FORCE=true
      shift 1
      ;;
    --yes)
      YES=true
      shift 1
      ;;
    *)
      printf 'Unknown option `%s`.\n' "$1" >&2
      exit 1
      ;;
  esac
done

OPERATING_SYSTEM="$(normalize_os)"
if [ "$OPERATING_SYSTEM" = unsupported ]; then
  print_line 'install.sh currently supports Linux and macOS hosts only. Use install.ps1 on Windows.' >&2
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
  print_line 'tar is required to install pyenv-native on POSIX hosts.' >&2
  exit 1
fi

mkdir -p "$(dirname -- "$INSTALL_ROOT")"
INSTALL_ROOT="$(CDPATH= cd -- "$(dirname -- "$INSTALL_ROOT")" && pwd)/$(basename -- "$INSTALL_ROOT")"
mkdir -p "$TEMP_ROOT"
TEMP_ROOT="$(CDPATH= cd -- "$TEMP_ROOT" && pwd)"
UPDATE_PROFILE_EFFECTIVE=false
if [ "$SHELL_KIND" != none ] && [ "$UPDATE_SHELL_PROFILE" = true ]; then
  UPDATE_PROFILE_EFFECTIVE=true
fi

assert_install_root_access
warn_existing_path_command
assert_install_root_state

ASSET_INFO="$(resolve_release_urls "$OPERATING_SYSTEM" "$ARCHITECTURE")"
BUNDLE_URL="$(printf '%s' "$ASSET_INFO" | sed -n '1p')"
CHECKSUM_URL="$(printf '%s' "$ASSET_INFO" | sed -n '2p')"
ASSET_NAME="$(printf '%s' "$ASSET_INFO" | sed -n '3p')"
SOURCE_LABEL="$(printf '%s' "$ASSET_INFO" | sed -n '4p')"
ensure_log_ready
emit_summary
confirm_action

DOWNLOAD_ROOT="$(mktemp -d "${TEMP_ROOT}/downloads-XXXXXX")"
EXTRACT_ROOT="$(mktemp -d "${TEMP_ROOT}/extract-XXXXXX")"
if [ "$KEEP_DOWNLOADS" != true ]; then
  trap 'cleanup_paths "$DOWNLOAD_ROOT" "$EXTRACT_ROOT"' EXIT HUP INT TERM
fi

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
MCP_EXECUTABLE_PATH="${EXTRACT_ROOT}/pyenv-mcp"
MANIFEST_PATH="${EXTRACT_ROOT}/bundle-manifest.json"

for required_path in "$INSTALLER_PATH" "$EXECUTABLE_PATH" "$MANIFEST_PATH"; do
  if [ ! -f "$required_path" ]; then
    printf 'Downloaded bundle was missing required file `%s`.
' "$required_path" >&2
    exit 1
  fi
done

chmod +x "$INSTALLER_PATH" "$EXECUTABLE_PATH"
if ! grep -q '"platform"[[:space:]]*:[[:space:]]*"'$OPERATING_SYSTEM'"' "$MANIFEST_PATH"; then
  printf 'Downloaded bundle manifest did not match host platform `%s`.
' "$OPERATING_SYSTEM" >&2
  exit 1
fi
if grep -q '"mcp_executable"' "$MANIFEST_PATH"; then
  if [ ! -f "$MCP_EXECUTABLE_PATH" ]; then
    printf 'Downloaded bundle declared an MCP server binary but `%s` was missing.
' "$MCP_EXECUTABLE_PATH" >&2
    exit 1
  fi
  chmod +x "$MCP_EXECUTABLE_PATH"
fi

write_step "Running bundled installer from ${INSTALLER_PATH}"
if [ "$FORCE" = true ]; then
  if [ -f "$MCP_EXECUTABLE_PATH" ]; then
    "$INSTALLER_PATH"       --source-path "$EXECUTABLE_PATH"       --source-mcp-path "$MCP_EXECUTABLE_PATH"       --install-root "$INSTALL_ROOT"       --shell "$SHELL_KIND"       --add-to-user-path "$ADD_TO_USER_PATH"       --update-shell-profile "$UPDATE_SHELL_PROFILE"       --refresh-shims "$REFRESH_SHIMS"       --log-path "$LOG_PATH"       --yes       --force
  else
    "$INSTALLER_PATH"       --source-path "$EXECUTABLE_PATH"       --install-root "$INSTALL_ROOT"       --shell "$SHELL_KIND"       --add-to-user-path "$ADD_TO_USER_PATH"       --update-shell-profile "$UPDATE_SHELL_PROFILE"       --refresh-shims "$REFRESH_SHIMS"       --log-path "$LOG_PATH"       --yes       --force
  fi
else
  if [ -f "$MCP_EXECUTABLE_PATH" ]; then
    "$INSTALLER_PATH"       --source-path "$EXECUTABLE_PATH"       --source-mcp-path "$MCP_EXECUTABLE_PATH"       --install-root "$INSTALL_ROOT"       --shell "$SHELL_KIND"       --add-to-user-path "$ADD_TO_USER_PATH"       --update-shell-profile "$UPDATE_SHELL_PROFILE"       --refresh-shims "$REFRESH_SHIMS"       --log-path "$LOG_PATH"       --yes
  else
    "$INSTALLER_PATH"       --source-path "$EXECUTABLE_PATH"       --install-root "$INSTALL_ROOT"       --shell "$SHELL_KIND"       --add-to-user-path "$ADD_TO_USER_PATH"       --update-shell-profile "$UPDATE_SHELL_PROFILE"       --refresh-shims "$REFRESH_SHIMS"       --log-path "$LOG_PATH"       --yes
  fi
fi

write_step 'Network install completed successfully.'
print_line ""
print_line "Installed pyenv-native to $INSTALL_ROOT"
print_line "Installed command: ${INSTALL_ROOT}/bin/pyenv"
print_line "Log file: $LOG_PATH"
if [ -n "$GITHUB_REPO" ]; then
  print_line "Remote uninstall helper: https://raw.githubusercontent.com/${GITHUB_REPO}/main/uninstall.sh"
fi
