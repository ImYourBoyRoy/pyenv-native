#!/usr/bin/env sh
# ./scripts/install-pyenv-native.sh
# Purpose: Installs the native pyenv executable into a portable POSIX root and optionally updates shell profile integration.
# How to run: sh ./scripts/install-pyenv-native.sh [--source-path <pyenv>] [--install-root <dir>] [--shell <bash|zsh|fish|sh|none>]
# Inputs: Optional source binary path, install root, shell preference, PATH/profile toggles, and a force-overwrite flag.
# Outputs/side effects: Copies the pyenv binary into <install-root>/bin, creates shims/versions/cache folders, and optionally appends shell init to the chosen profile.
# Notes: Keeps the install portable and registry-free; profile updates are the POSIX equivalent of PATH integration.

set -eu

SOURCE_PATH=""
INSTALL_ROOT="${HOME}/.pyenv"
SHELL_KIND="sh"
ADD_TO_USER_PATH="true"
UPDATE_SHELL_PROFILE="true"
REFRESH_SHIMS="true"
FORCE="false"

parse_bool() {
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|on) printf '%s\n' "true" ;;
    0|false|no|off) printf '%s\n' "false" ;;
    *) printf 'Invalid boolean value `%s`\n' "$1" >&2; exit 1 ;;
  esac
}

resolve_script_dir() {
  CDPATH= cd -- "$(dirname -- "$0")" && pwd
}

resolve_source_binary() {
  if [ -n "$SOURCE_PATH" ] && [ -f "$SOURCE_PATH" ]; then
    printf '%s\n' "$SOURCE_PATH"
    return
  fi

  script_dir="$(resolve_script_dir)"
  for candidate in \
    "$script_dir/../target/release/pyenv" \
    "$script_dir/../target/debug/pyenv"
  do
    if [ -f "$candidate" ]; then
      printf '%s\n' "$candidate"
      return
    fi
  done

  printf '%s\n' "pyenv-native source binary was not found. Pass --source-path <pyenv> or build the project first." >&2
  exit 1
}

profile_path_for_shell() {
  case "$1" in
    bash) printf '%s\n' "${HOME}/.bashrc" ;;
    zsh) printf '%s\n' "${HOME}/.zshrc" ;;
    fish) printf '%s\n' "${HOME}/.config/fish/config.fish" ;;
    sh) printf '%s\n' "${HOME}/.profile" ;;
    none) printf '%s\n' "" ;;
    *) printf '%s\n' "Unsupported shell \`$1\`" >&2; exit 1 ;;
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
      printf '%s\n' ""
      ;;
    *)
      cat <<EOF
$begin_marker
if [ -x '$installed_exe' ]; then
  case ":\${PATH}:" in
    *:'$install_bin':*) ;;
    *) export PATH='$install_bin':"\${PATH}" ;;
  esac
  eval "\$('$installed_exe' init - $shell_kind)"
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
          if (NR > 0) { print "" }
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

while [ "$#" -gt 0 ]; do
  case "$1" in
    --source-path)
      SOURCE_PATH="${2:-}"
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

RESOLVED_SOURCE="$(resolve_source_binary)"
INSTALL_ROOT="$(cd "$(dirname -- "$INSTALL_ROOT")" 2>/dev/null && pwd)/$(basename -- "$INSTALL_ROOT")"
INSTALL_BIN="${INSTALL_ROOT}/bin"
INSTALLED_EXE="${INSTALL_BIN}/pyenv"

mkdir -p "$INSTALL_BIN" "${INSTALL_ROOT}/shims" "${INSTALL_ROOT}/versions" "${INSTALL_ROOT}/cache"

if [ -e "$INSTALLED_EXE" ] && [ "$FORCE" != "true" ]; then
  printf 'pyenv-native is already installed at %s. Re-run with --force to overwrite.\n' "$INSTALLED_EXE" >&2
  exit 1
fi

cp -f "$RESOLVED_SOURCE" "$INSTALLED_EXE"
chmod +x "$INSTALLED_EXE"

if [ "$REFRESH_SHIMS" = "true" ]; then
  PYENV_ROOT="$INSTALL_ROOT" "$INSTALLED_EXE" rehash >/dev/null 2>&1 || true
fi

PROFILE_PATH=""
if [ "$UPDATE_SHELL_PROFILE" = "true" ] && [ "$SHELL_KIND" != "none" ]; then
  PROFILE_PATH="$(profile_path_for_shell "$SHELL_KIND")"
  PROFILE_BLOCK="$(render_profile_block "$INSTALLED_EXE" "$SHELL_KIND" "$INSTALL_BIN")"
  update_profile_block "$PROFILE_PATH" "$PROFILE_BLOCK"
fi

printf 'source_binary: %s\n' "$RESOLVED_SOURCE"
printf 'install_root: %s\n' "$INSTALL_ROOT"
printf 'installed_exe: %s\n' "$INSTALLED_EXE"
printf 'install_bin: %s\n' "$INSTALL_BIN"
printf 'shell: %s\n' "$SHELL_KIND"
printf 'add_to_user_path: %s\n' "$ADD_TO_USER_PATH"
printf 'update_shell_profile: %s\n' "$UPDATE_SHELL_PROFILE"
printf 'refresh_shims: %s\n' "$REFRESH_SHIMS"
if [ -n "$PROFILE_PATH" ]; then
  printf 'profile_path: %s\n' "$PROFILE_PATH"
fi

if [ "$ADD_TO_USER_PATH" = "true" ] && [ "$UPDATE_SHELL_PROFILE" != "true" ]; then
  printf '\nPATH note: on POSIX systems, persistent PATH integration normally happens through your shell profile.\n'
fi
