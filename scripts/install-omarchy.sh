#!/usr/bin/env bash
# Build Neko Legends Control Center for Linux and install it for the current
# user so it shows up in the Omarchy app launcher (Super+Space).
#
#   scripts/install-omarchy.sh             build + install
#   scripts/install-omarchy.sh --uninstall remove the launcher entry and binary
#
# Installs to ~/.local (no sudo). Apps downloaded by the Control Center live in
# ~/.local/share/NekoLegends/apps and are left alone by --uninstall.
set -euo pipefail

APP_ID="neko-legends-control-center"
APP_NAME="Neko Legends Control Center"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
DESKTOP_FILE="${DATA_DIR}/applications/${APP_ID}.desktop"
ICON_FILE="${DATA_DIR}/icons/hicolor/256x256/apps/${APP_ID}.png"

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -f "${BIN_DIR}/${APP_ID}" "${DESKTOP_FILE}" "${ICON_FILE}"
  update-desktop-database "${DATA_DIR}/applications" 2>/dev/null || true
  echo "Removed ${APP_NAME}."
  exit 0
fi

# Rust: use cargo from PATH, or fall back to mise's stable toolchain.
if cargo --version >/dev/null 2>&1; then
  RUN=()
elif command -v mise >/dev/null 2>&1; then
  mise install -q rust@stable
  RUN=(mise exec rust@stable --)
else
  echo "Rust is required: install it with 'mise use -g rust' or 'sudo pacman -S rust'." >&2
  exit 1
fi

for lib in webkit2gtk-4.1 gtk+-3.0 libsoup-3.0; do
  if ! pkg-config --exists "$lib"; then
    echo "Missing system library '$lib'. Install: sudo pacman -S --needed webkit2gtk-4.1 gtk3 libsoup3" >&2
    exit 1
  fi
done

cd "$ROOT"
[[ -d node_modules ]] || npm ci
# A node_modules installed from Windows (shared drive) only has the win32 native
# binaries, and npm prunes them if asked to add Linux ones. Unpack the Linux
# builds beside them so both OSes keep working.
for pkg in @esbuild/linux-x64 @rollup/rollup-linux-x64-gnu @tauri-apps/cli-linux-x64-gnu; do
  [[ -d "node_modules/$pkg" ]] && continue
  ver="$(node -p "require('./package-lock.json').packages['node_modules/$pkg'].version")"
  tmp="$(mktemp -d)"
  (cd "$tmp" && npm pack --silent "$pkg@$ver" >/dev/null)
  mkdir -p "node_modules/$pkg"
  tar -xzf "$tmp"/*.tgz -C "node_modules/$pkg" --strip-components=1
  rm -rf "$tmp"
done
"${RUN[@]}" npx tauri build --no-bundle

install -Dm755 "src-tauri/target/release/${APP_ID}" "${BIN_DIR}/${APP_ID}"

mkdir -p "$(dirname "$ICON_FILE")"
if command -v magick >/dev/null 2>&1; then
  magick src-tauri/icons/icon.png -resize 256x256 "$ICON_FILE"
else
  install -Dm644 src-tauri/icons/icon.png "$ICON_FILE"
fi

mkdir -p "$(dirname "$DESKTOP_FILE")"
cat >"$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=${APP_NAME}
Comment=Launcher and update hub for Neko Legends apps
Exec=${BIN_DIR}/${APP_ID}
Icon=${APP_ID}
Terminal=false
Categories=Utility;Development;
StartupWMClass=${APP_ID}
EOF

update-desktop-database "${DATA_DIR}/applications" 2>/dev/null || true
gtk-update-icon-cache -q "${DATA_DIR}/icons/hicolor" 2>/dev/null || true

echo "Installed ${APP_NAME}. Open it from the app launcher (Super+Space)."
