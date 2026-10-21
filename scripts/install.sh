#!/bin/sh
# Install easyscript, the app and its command line, from the latest release on GitHub. For
# you alone: no administrator, and nothing to install first.
#
#   curl -fsSL https://raw.githubusercontent.com/murparreira/easyscript/main/scripts/install.sh | sh
#
# Run it again to update. macOS: the app goes to ~/Applications. Linux (x86_64): to
# ~/.local, with a menu entry; PREFIX changes where. The command line `easyscript` goes to
# ~/.local/bin (PREFIX/bin). EASYSCRIPT_VERSION=v0.1.0 picks a release instead of the latest.
set -eu

REPO="${EASYSCRIPT_REPO:-murparreira/easyscript}"
API="${EASYSCRIPT_API:-https://api.github.com}"

say() { printf '\n\033[1m%s\033[0m\n' "$*"; }
fail() {
  printf '\n\033[1;31merror:\033[0m %s\n' "$*" >&2
  exit 1
}
need() { command -v "$1" >/dev/null 2>&1 || fail "$1 is needed and is not here"; }
need curl
need tar

os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Darwin) ;;
  Linux) [ "$arch" = x86_64 ] || fail "there is no easyscript for Linux on $arch yet, only x86_64" ;;
  *) fail "on Windows, run in PowerShell: irm https://raw.githubusercontent.com/$REPO/main/scripts/install.ps1 | iex" ;;
esac

say "Finding the release"
if [ -n "${EASYSCRIPT_VERSION:-}" ]; then
  query="$API/repos/$REPO/releases/tags/$EASYSCRIPT_VERSION"
else
  # The newest published release, pre-releases included.
  query="$API/repos/$REPO/releases?per_page=1"
fi
release="$(curl -fsSL -H 'Accept: application/vnd.github+json' "$query")" ||
  fail "cannot reach GitHub to find the release ($query)"
urls="$(printf '%s' "$release" | grep -o '"browser_download_url": *"[^"]*"' | sed 's/.*"\(http[^"]*\)"$/\1/')"
tag="$(printf '%s' "$release" | grep -o '"tag_name": *"[^"]*"' | head -n 1 | sed 's/.*"\([^"]*\)"$/\1/')"
[ -n "$urls" ] || fail "no release of $REPO has files to install yet"
pick() {
  found="$(printf '%s\n' "$urls" | grep -E "$1" | head -n 1)"
  [ -n "$found" ] || fail "the release $tag has no $2"
  printf '%s' "$found"
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fetch() { curl -fL --progress-bar -o "$2" "$1" || fail "cannot download $1"; }

if [ "$os" = Darwin ]; then
  app_url="$(pick '_universal\.dmg$' 'app for macOS')"
  cli_url="$(pick 'easyscript-cli-macos-(universal|arm64)\.tar\.gz$' 'command line for macOS')"
  say "Downloading easyscript $tag"
  fetch "$app_url" "$tmp/easyscript.dmg"
  fetch "$cli_url" "$tmp/cli.tar.gz"
  say "Installing in ~/Applications"
  hdiutil attach -nobrowse -quiet -mountpoint "$tmp/disk" "$tmp/easyscript.dmg"
  mkdir -p "$HOME/Applications"
  rm -rf "$HOME/Applications/easyscript.app"
  ditto "$tmp/disk/easyscript.app" "$HOME/Applications/easyscript.app"
  hdiutil detach -quiet "$tmp/disk"
  # Downloaded with curl, it carries no quarantine; this makes sure.
  xattr -dr com.apple.quarantine "$HOME/Applications/easyscript.app" 2>/dev/null || true
  bin="$HOME/.local/bin"
  mkdir -p "$bin"
  tar -xzf "$tmp/cli.tar.gz" -C "$tmp"
  cp "$tmp"/easyscript-cli-*/easyscript "$bin/easyscript"
  chmod 755 "$bin/easyscript"
  echo "The app:          easyscript in ~/Applications (Launchpad, Spotlight)"
  echo "The command line: $bin/easyscript"
else
  app_url="$(pick '_amd64\.AppImage$' 'app for Linux')"
  cli_url="$(pick 'easyscript-cli-linux-x64\.tar\.gz$' 'command line for Linux')"
  say "Downloading easyscript $tag"
  fetch "$app_url" "$tmp/easyscript.AppImage"
  fetch "$cli_url" "$tmp/cli.tar.gz"
  prefix="${PREFIX:-$HOME/.local}"
  say "Installing in $prefix"
  # Unpacked once, so it runs without FUSE, which many systems lack.
  chmod +x "$tmp/easyscript.AppImage"
  (cd "$tmp" && ./easyscript.AppImage --appimage-extract >/dev/null) || fail "cannot unpack the app"
  mkdir -p "$prefix/lib" "$prefix/bin" "$prefix/share/applications" "$prefix/share/icons/hicolor/128x128/apps"
  rm -rf "$prefix/lib/easyscript"
  mv "$tmp/squashfs-root" "$prefix/lib/easyscript"
  cat >"$prefix/bin/easyscript-app" <<EOF
#!/bin/sh
exec "$prefix/lib/easyscript/AppRun" "\$@"
EOF
  chmod 755 "$prefix/bin/easyscript-app"
  cp "$prefix/lib/easyscript/easyscript.png" "$prefix/share/icons/hicolor/128x128/apps/easyscript.png" 2>/dev/null || true
  cat >"$prefix/share/applications/easyscript.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=easyscript
Comment=Write a game's dialogue where it lives
Exec="$prefix/bin/easyscript-app" %f
Icon=easyscript
StartupWMClass=easyscript-app
Terminal=false
Categories=Development;
EOF
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$prefix/share/applications" >/dev/null 2>&1 || true
  bin="$prefix/bin"
  tar -xzf "$tmp/cli.tar.gz" -C "$tmp"
  cp "$tmp"/easyscript-cli-*/easyscript "$bin/easyscript"
  chmod 755 "$bin/easyscript"
  echo "The app:          easyscript in your applications menu, or easyscript-app [a game folder]"
  echo "The command line: $bin/easyscript"
fi
case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "To run the command line by name, add $bin to your PATH." ;;
esac
say "Done: easyscript $tag. Run this again to update."
