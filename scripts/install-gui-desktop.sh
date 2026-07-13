#!/bin/sh
# ./scripts/install-gui-desktop.sh
# Install Freedesktop launcher + hicolor icons for pyenv-gui on Linux.

set -eu

APP_ID="com.pyenv-native.gui"
WM_CLASS="pyenv-gui"
GUI_EXE="${1:-}"
ICONS_SRC="${2:-}"

if [ -z "$GUI_EXE" ]; then
  printf 'Usage: %s /path/to/pyenv-gui [icons_source_dir]\n' "$0" >&2
  exit 1
fi

if [ ! -x "$GUI_EXE" ]; then
  printf 'GUI executable not found or not executable: %s\n' "$GUI_EXE" >&2
  exit 1
fi

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)"
GUI_BIN_DIR="$(CDPATH= cd -- "$(dirname "$GUI_EXE")" && pwd)"
INSTALL_SHARE="$(CDPATH= cd -- "$GUI_BIN_DIR/.." && pwd)/share"

if [ -z "$ICONS_SRC" ]; then
  if [ -d "$INSTALL_SHARE/icons/hicolor" ]; then
    ICONS_SRC="$INSTALL_SHARE/icons/hicolor"
    USE_PRESTAGED_ICONS="true"
  elif [ -d "$REPO_ROOT/crates/pyenv-gui/icons" ]; then
    ICONS_SRC="$REPO_ROOT/crates/pyenv-gui/icons"
  elif [ -d "$GUI_BIN_DIR/share/icons/hicolor" ]; then
    ICONS_SRC="$GUI_BIN_DIR/share/icons/hicolor"
    USE_PRESTAGED_ICONS="true"
  else
    printf 'Icon source directory not found. Pass icons_source_dir as the second argument.\n' >&2
    exit 1
  fi
fi

DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="$DATA_HOME/applications"
ICONS_DIR="$DATA_HOME/icons/hicolor"
DESKTOP_PATH="$APPS_DIR/${APP_ID}.desktop"
ICON_FILE="$ICONS_DIR/128x128/apps/${APP_ID}.png"

mkdir -p "$APPS_DIR"

install_icon() {
  size="$1"
  src_name="$2"
  dest_dir="$ICONS_DIR/${size}/apps"
  mkdir -p "$dest_dir"
  if [ -f "$ICONS_SRC/$src_name" ]; then
    cp -f "$ICONS_SRC/$src_name" "$dest_dir/${APP_ID}.png"
  fi
}

if [ "${USE_PRESTAGED_ICONS:-false}" = "true" ]; then
  for themed in "$ICONS_SRC"/*/apps/"${APP_ID}.png"; do
    if [ -f "$themed" ]; then
      size_dir="$(basename "$(dirname "$(dirname "$themed")")")"
      mkdir -p "$ICONS_DIR/$size_dir/apps"
      cp -f "$themed" "$ICONS_DIR/$size_dir/apps/${APP_ID}.png"
    fi
  done
else
  install_icon "32x32" "32x32.png"
  install_icon "128x128" "128x128.png"
  install_icon "256x256" "128x128@2x.png"
  install_icon "512x512" "icon.png"
fi

ICON_VALUE="$APP_ID"
if [ -f "$ICON_FILE" ]; then
  ICON_VALUE="$ICON_FILE"
fi

cat > "$DESKTOP_PATH" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Pyenv Native
GenericName=Python Environment Manager
Comment=Manage Python versions and virtual environments
Exec=$GUI_EXE
Icon=$ICON_VALUE
StartupWMClass=$WM_CLASS
Categories=Development;Utility;
Terminal=false
EOF

chmod 644 "$DESKTOP_PATH"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$ICONS_DIR" >/dev/null 2>&1 || true
fi

printf 'Installed desktop launcher: %s\n' "$DESKTOP_PATH"
printf 'Installed icon theme entries under: %s\n' "$ICONS_DIR"
