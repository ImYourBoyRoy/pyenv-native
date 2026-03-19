#!/usr/bin/env sh
# ./scripts/install-pyenv-native.sh
# Purpose: Installs the native pyenv executables into a portable POSIX root and optionally updates shell profile integration.
# How to run: sh ./scripts/install-pyenv-native.sh [--source-path <pyenv>] [--source-mcp-path <pyenv-mcp>] [--install-root <dir>] [--shell <bash|zsh|fish|sh|none>] [--yes]
# Inputs: Optional source binary paths, install root, shell preference, PATH/profile toggles, logging location, and a force-overwrite flag.
# Outputs/side effects: Copies pyenv plus pyenv-mcp into <install-root>/bin, creates shims/versions/cache folders, optionally appends shell init to the chosen profile, and writes an install log.
# Notes: Keeps the install portable and registry-free; profile updates are the POSIX equivalent of PATH integration.

set -eu

SOURCE_PATH=""
SOURCE_MCP_PATH=""
INSTALL_ROOT="${HOME}/.pyenv"
SHELL_KIND=""
ADD_TO_USER_PATH="true"
UPDATE_SHELL_PROFILE="true"
REFRESH_SHIMS="true"
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

is_interactive() {
  [ -r /dev/tty ]
}

resolve_script_dir() {
  CDPATH= cd -- "$(dirname -- "$0")" && pwd
}

resolve_binary_path() {
  explicit_path="$1"
  binary_label="$2"
  required="$3"
  shift 3

  if [ -n "$explicit_path" ] && [ -f "$explicit_path" ]; then
    print_line "$explicit_path"
    return 0
  fi

  for candidate in "$@"; do
    if [ -n "$candidate" ] && [ -f "$candidate" ]; then
      print_line "$candidate"
      return 0
    fi
  done

  if [ "$required" = "true" ]; then
    printf 'pyenv-native source binary `%s` was not found. Pass an explicit path or build the project first.\n' "$binary_label" >&2
    exit 1
  fi

  return 1
}

resolve_source_binary() {
  script_dir="$(resolve_script_dir)"
  resolve_binary_path "$SOURCE_PATH" 'pyenv' true \
    "$script_dir/../target/release/pyenv" \
    "$script_dir/../target/debug/pyenv"
}

resolve_source_mcp_binary() {
  script_dir="$(resolve_script_dir)"
  sibling_dir="$(dirname -- "$RESOLVED_SOURCE")"
  resolve_binary_path "$SOURCE_MCP_PATH" 'pyenv-mcp' false \
    "$sibling_dir/pyenv-mcp" \
    "$script_dir/../target/release/pyenv-mcp" \
    "$script_dir/../target/debug/pyenv-mcp" || true
}

profile_path_for_shell() {
  case "$1" in
    bash) print_line "${HOME}/.bashrc" ;;
    zsh) print_line "${HOME}/.zshrc" ;;
    fish) print_line "${HOME}/.config/fish/config.fish" ;;
    sh)
      if is_termux; then
        print_line "${HOME}/.bashrc"
      else
        print_line "${HOME}/.profile"
      fi
      ;;
    none) print_line "" ;;
    *)
      printf 'Unsupported shell `%s`.\n' "$1" >&2
      exit 1
      ;;
  esac
}

render_reload_hint() {
  profile_path="$1"
  shell_kind="$2"
  [ -n "$profile_path" ] || return 0

  case "$shell_kind" in
    fish) printf 'source %s\n' "$profile_path" ;;
    *) printf '. %s\n' "$profile_path" ;;
  esac
}

render_profile_block() {
  installed_exe="$1"
  shell_kind="$2"
  install_bin="$3"
  begin_marker="# >>> pyenv-native init >>>"
  end_marker="# <<< pyenv-native init <<<"

  case "$shell_kind" in
    fish)
      cat <<EOF
$begin_marker
if test -x '$installed_exe'
  if not contains -- '$install_bin' \$PATH
    set -gx PATH '$install_bin' \$PATH
  end
  '$installed_exe' init - fish | source
end
$end_marker
EOF
      ;;
    none)
      print_line ""
      ;;
    *)
      cat <<EOF
$begin_marker
if [ -x '$installed_exe' ]; then
  case ":\${PATH}:" in
    *:'$install_bin':*) ;;
    *) export PATH='$install_bin':"\${PATH}" ;;
  esac
  eval "\$('${installed_exe}' init - $shell_kind)"
fi
$end_marker
EOF
      ;;
  esac
}

update_profile_block() {
  profile_path="$1"
  block="$2"

  [ -n "$profile_path" ] || return 0

  profile_dir="$(dirname -- "$profile_path")"
  mkdir -p "$profile_dir"

  if [ -f "$profile_path" ]; then
    existing="$(cat "$profile_path")"
  else
    existing=""
  fi

  begin_marker="# >>> pyenv-native init >>>"
  end_marker="# <<< pyenv-native init <<<"

  if printf '%s' "$existing" | grep -Fq "$begin_marker"; then
    updated="$(printf '%s\n' "$existing" | awk -v begin="$begin_marker" -v end="$end_marker" -v block="$block" '
      BEGIN { skipping = 0; replaced = 0 }
      index($0, begin) == 1 && replaced == 0 {
        print block
        skipping = 1
        replaced = 1
        next
      }
      skipping == 1 {
        if (index($0, end) == 1) {
          skipping = 0
        }
        next
      }
      { print }
      END {
        if (replaced == 0) {
          if (NR > 0) {
            print ""
          }
          print block
        }
      }
    ')"
  else
    if [ -n "$existing" ]; then
      updated="${existing}

${block}
"
    else
      updated="${block}
"
    fi
  fi

  printf '%s\n' "$updated" > "$profile_path"
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
    printf 'pyenv-native is already installed at %s. Re-run with --force to overwrite or uninstall first.\n' "$installed_executable" >&2
    exit 1
  fi

  if [ -d "$INSTALL_ROOT" ] \
    && [ ! -e "$installed_executable" ] \
    && [ "$FORCE" != "true" ]; then
    non_log_children="$(find "$INSTALL_ROOT" -mindepth 1 -maxdepth 1 -exec basename {} \; 2>/dev/null | grep -v '^logs$' || true)"
    if [ -n "$non_log_children" ]; then
      printf 'Install root `%s` already exists and is not empty. Re-run with --force or choose a different --install-root.\n' "$INSTALL_ROOT" >&2
      exit 1
    fi
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

ensure_log_ready() {
  if [ -z "$LOG_PATH" ]; then
    timestamp="$(date +%Y%m%d-%H%M%S 2>/dev/null || print_line unknown)"
    LOG_PATH="${INSTALL_ROOT}/logs/install-${timestamp}.log"
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

write_warn() {
  log_line WARN "$1"
}

emit_summary() {
  print_line ""
  print_line "pyenv-native install summary"
  print_line "============================"
  print_line "source binary : $RESOLVED_SOURCE"
  print_line "source mcp    : ${RESOLVED_MCP_SOURCE:-<not found>}"
  print_line "install root  : $INSTALL_ROOT"
  print_line "installed exe : $INSTALLED_EXE"
  print_line "installed mcp : $INSTALLED_MCP_EXE"
  print_line "shell         : $SHELL_KIND"
  print_line "profile update: $UPDATE_PROFILE_EFFECTIVE"
  print_line "path hint     : $ADD_TO_USER_PATH"
  print_line "refresh shims : $REFRESH_SHIMS"
  print_line "force         : $FORCE"
  print_line "log path      : $LOG_PATH"
  print_line ""
  print_line "This will create or update a portable pyenv-native installation under the selected root."
  print_line "It installs pyenv plus the agent-friendly pyenv-mcp server when available, writes an install log, and runs basic sanity checks."
  if [ "$UPDATE_PROFILE_EFFECTIVE" = "true" ]; then
    print_line "Your shell profile will be updated so future sessions can find pyenv-native automatically."
  else
    print_line "No shell profile changes will be made."
  fi
  print_line ""
}

confirm_action() {
  if [ "$YES" = "true" ]; then
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

run_sanity_check() {
  command_path="$1"
  name="$2"
  shift 2

  if output="$(PYENV_ROOT="$INSTALL_ROOT" "$command_path" "$@" 2>&1)"; then
    first_line="$(printf '%s' "$output" | awk 'NR == 1 { print; exit }')"
    if [ -n "$first_line" ]; then
      log_line INFO "Sanity check passed: $name -> $first_line"
    else
      log_line INFO "Sanity check passed: $name"
    fi
    return 0
  fi

  log_line ERROR "Sanity check failed: $name"
  print_line "$output" >&2
  if [ -n "$LOG_PATH" ]; then
    print_line "$output" >> "$LOG_PATH"
  fi
  exit 1
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --source-path)
      SOURCE_PATH="${2:-}"
      shift 2
      ;;
    --source-mcp-path)
      SOURCE_MCP_PATH="${2:-}"
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
    --log-path)
      LOG_PATH="${2:-}"
      shift 2
      ;;
    --force)
      FORCE="true"
      shift 1
      ;;
    --yes)
      YES="true"
      shift 1
      ;;
    *)
      printf 'Unknown option `%s`.\n' "$1" >&2
      exit 1
      ;;
  esac
done

if [ -z "$SHELL_KIND" ]; then
  SHELL_KIND="$(detect_shell_kind)"
fi

mkdir -p "$(dirname -- "$INSTALL_ROOT")"
INSTALL_ROOT="$(CDPATH= cd -- "$(dirname -- "$INSTALL_ROOT")" && pwd)/$(basename -- "$INSTALL_ROOT")"
INSTALL_BIN="${INSTALL_ROOT}/bin"
INSTALLED_EXE="${INSTALL_BIN}/pyenv"
INSTALLED_MCP_EXE="${INSTALL_BIN}/pyenv-mcp"
RESOLVED_SOURCE="$(resolve_source_binary)"
RESOLVED_MCP_SOURCE="$(resolve_source_mcp_binary)"
UPDATE_PROFILE_EFFECTIVE="false"
if [ "$SHELL_KIND" != "none" ] && [ "$UPDATE_SHELL_PROFILE" = "true" ]; then
  UPDATE_PROFILE_EFFECTIVE="true"
fi

assert_install_root_access
assert_install_root_state
warn_existing_path_command
ensure_log_ready
emit_summary
confirm_action

write_step "Creating portable pyenv-native layout"
mkdir -p "$INSTALL_BIN" "${INSTALL_ROOT}/shims" "${INSTALL_ROOT}/versions" "${INSTALL_ROOT}/cache" "${INSTALL_ROOT}/logs"
cp -f "$RESOLVED_SOURCE" "$INSTALLED_EXE"
chmod +x "$INSTALLED_EXE"

if [ -n "$RESOLVED_MCP_SOURCE" ] && [ -f "$RESOLVED_MCP_SOURCE" ]; then
  cp -f "$RESOLVED_MCP_SOURCE" "$INSTALLED_MCP_EXE"
  chmod +x "$INSTALLED_MCP_EXE"
  write_step "Installed MCP server binary into ${INSTALLED_MCP_EXE}"
else
  write_warn 'pyenv-mcp source binary was not found; installing pyenv CLI only.'
fi

PROFILE_PATH=""
if [ "$UPDATE_PROFILE_EFFECTIVE" = "true" ]; then
  PROFILE_PATH="$(profile_path_for_shell "$SHELL_KIND")"
  PROFILE_BLOCK="$(render_profile_block "$INSTALLED_EXE" "$SHELL_KIND" "$INSTALL_BIN")"
  update_profile_block "$PROFILE_PATH" "$PROFILE_BLOCK"
  write_step "Updated shell profile at ${PROFILE_PATH}"
fi

if [ "$REFRESH_SHIMS" = "true" ]; then
  PYENV_ROOT="$INSTALL_ROOT" "$INSTALLED_EXE" rehash >/dev/null 2>&1
  write_step 'Refreshed shims'
fi

run_sanity_check "$INSTALLED_EXE" 'pyenv --version' --version
run_sanity_check "$INSTALLED_EXE" 'pyenv root' root
run_sanity_check "$INSTALLED_EXE" 'pyenv commands' commands
if [ -f "$INSTALLED_MCP_EXE" ]; then
  run_sanity_check "$INSTALLED_MCP_EXE" 'pyenv-mcp guide' guide
fi

if [ "$ADD_TO_USER_PATH" = "true" ] && [ "$UPDATE_PROFILE_EFFECTIVE" != "true" ]; then
  write_warn 'Persistent PATH integration on POSIX systems usually happens through your shell profile. Add your install bin manually if needed.'
fi

if [ "$UPDATE_PROFILE_EFFECTIVE" = "true" ] && [ -n "$PROFILE_PATH" ]; then
  RELOAD_HINT="$(render_reload_hint "$PROFILE_PATH" "$SHELL_KIND")"
  if [ -n "$RELOAD_HINT" ]; then
    write_step "Open a new shell or run: $RELOAD_HINT"
  fi
fi

write_step 'Install completed successfully.'
log_line INFO "Final artifacts:"
log_line INFO "  install_root: $INSTALL_ROOT"
log_line INFO "  installed_exe: $INSTALLED_EXE"
if [ -f "$INSTALLED_MCP_EXE" ]; then
  log_line INFO "  installed_mcp: $INSTALLED_MCP_EXE"
fi
log_line INFO "  log_path: $LOG_PATH"
if [ -n "$PROFILE_PATH" ]; then
  log_line INFO "  profile_path: $PROFILE_PATH"
fi
print_line ""
print_line "Installed pyenv-native to $INSTALL_ROOT"
print_line "Installed command: ${INSTALL_ROOT}/bin/pyenv"
if [ -f "$INSTALLED_MCP_EXE" ]; then
  print_line "Installed MCP server: ${INSTALLED_MCP_EXE}"
  print_line "MCP config helper: ${INSTALLED_MCP_EXE} print-config"
fi
print_line "Log file: $LOG_PATH"
