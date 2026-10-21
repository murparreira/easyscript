#!/usr/bin/env bash
# Build easyscript from this source tree and install it for you, in one command:
#
#   ./scripts/install.sh            build, then install the app (and the command line)
#   ./scripts/install.sh --no-install   build only; run it with target/release/easyscript-app
#
# macOS: the app goes to ~/Applications. Linux: to ~/.local (a menu entry, and
# `easyscript-app` on the PATH). The command line `easyscript` goes to ~/.local/bin.
# PREFIX changes where on Linux (default ~/.local).
#
# Needs Rust and Node 22: with mise (https://mise.jdx.dev) installed, the script gets both
# itself. On Linux it also needs the WebKitGTK libraries; it names the command to install
# them when they are missing. Nothing is installed with sudo.
set -euo pipefail
cd "$(dirname "$0")/.."

bold() { printf '\n\033[1m%s\033[0m\n' "$*"; }
fail() {
  printf '\n\033[1;31merror:\033[0m %s\n' "$*" >&2
  exit 1
}

install=1
for arg in "$@"; do
  case "$arg" in
    --no-install) install=0 ;;
    -h | --help)
      sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) fail "unknown option $arg (try --help)" ;;
  esac
done

os="$(uname -s)"
case "$os" in
  Linux | Darwin) ;;
  *) fail "on Windows, run scripts\\install.ps1 in PowerShell" ;;
esac

# Rust and Node: from mise when it is there (mise.toml pins them), else from the PATH.
if command -v mise >/dev/null 2>&1; then
  bold "Rust and Node, with mise"
  mise trust --quiet "$PWD/mise.toml" >/dev/null 2>&1 || true
  mise install
  eval "$(mise env --shell bash)"
fi
command -v cargo >/dev/null 2>&1 || fail "Rust is missing. Install it from https://rustup.rs, or install mise (https://mise.jdx.dev) and run this again."
command -v npm >/dev/null 2>&1 || fail "Node is missing. Install Node 22 from https://nodejs.org, or install mise (https://mise.jdx.dev) and run this again."
node_major="$(node -p 'process.versions.node.split(".")[0]')"
[ "$node_major" -ge 20 ] || fail "Node $node_major is too old: Node 22 is needed."

if [ "$os" = Linux ] && ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
  if command -v apt-get >/dev/null 2>&1; then
    hint="sudo apt-get install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev"
  elif command -v dnf >/dev/null 2>&1; then
    hint="sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel libxdo-devel && sudo dnf group install c-development"
  elif command -v pacman >/dev/null 2>&1; then
    hint="sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl appmenu-gtk-module libappindicator-gtk3 librsvg xdotool"
  elif command -v zypper >/dev/null 2>&1; then
    hint="sudo zypper in webkit2gtk3-devel libopenssl-devel curl wget file libappindicator3-1 librsvg-devel && sudo zypper in -t pattern devel_basis"
  else
    hint="see https://tauri.app/start/prerequisites/#linux"
  fi
  fail "the WebKitGTK libraries the app is built on are missing. Install them with:

  $hint

then run this again."
fi

bold "JavaScript packages"
npm ci --no-audit --no-fund

bold "The command line (a few minutes the first time)"
npm run build:page
cargo build --release --locked -p es-cli

bold "The app (a few minutes the first time)"
if [ "$os" = Darwin ]; then
  npm run tauri -- build --bundles app
else
  npm run tauri -- build --no-bundle
fi

if [ "$install" = 0 ]; then
  bold "Built"
  echo "The app:          target/release/easyscript-app [a game folder]"
  echo "The command line: target/release/easyscript"
  exit 0
fi

if [ "$os" = Darwin ]; then
  bold "Installing in ~/Applications"
  mkdir -p "$HOME/Applications" "$HOME/.local/bin"
  rm -rf "$HOME/Applications/easyscript.app"
  cp -R target/release/bundle/macos/easyscript.app "$HOME/Applications/"
  cp target/release/easyscript "$HOME/.local/bin/"
  echo "The app:          ~/Applications/easyscript.app (in Launchpad and Spotlight)"
  echo "The command line: ~/.local/bin/easyscript"
  bin="$HOME/.local/bin"
else
  prefix="${PREFIX:-$HOME/.local}"
  bold "Installing in $prefix"
  mkdir -p "$prefix/bin" "$prefix/share/applications" "$prefix/share/icons/hicolor/128x128/apps"
  install -m 755 target/release/easyscript-app "$prefix/bin/easyscript-app"
  install -m 755 target/release/easyscript "$prefix/bin/easyscript"
  install -m 644 app/src-tauri/icons/128x128.png "$prefix/share/icons/hicolor/128x128/apps/easyscript.png"
  cat >"$prefix/share/applications/easyscript.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=easyscript
Comment=Write a game's dialogue where it lives
Exec="$prefix/bin/easyscript-app" %f
Icon=easyscript
Terminal=false
Categories=Development;
EOF
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$prefix/share/applications" >/dev/null 2>&1 || true
  echo "The app:          easyscript in your applications menu, or easyscript-app [a game folder]"
  echo "The command line: easyscript"
  bin="$prefix/bin"
fi
case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "Add $bin to your PATH to run the command line by name." ;;
esac
bold "Done. Run this again to update after pulling new changes."
