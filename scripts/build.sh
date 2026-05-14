#!/usr/bin/env bash
set -e

BUILD_TYPE="${1:-dev}"

TILES_INSTALL_PATH=$(command -v tiles 2>/dev/null || echo "")

install_binary() {
  if [ -z "$TILES_INSTALL_PATH" ]; then
    echo "WARNING: tiles not found in PATH, skipping binary install"
    return
  fi

  local src="$HOME/.cargo/bin/tiles"
  if [ "$TILES_INSTALL_PATH" != "$src" ]; then
    local bak="${TILES_INSTALL_PATH}.bak"
    mv "$TILES_INSTALL_PATH" "$bak" 2>/dev/null || true
    cp "$src" "$TILES_INSTALL_PATH" 2>/dev/null || {
      mv "$bak" "$TILES_INSTALL_PATH" 2>/dev/null || true
      echo "WARNING: could not install to $TILES_INSTALL_PATH (binary in use)"
      return
    }
    rm -f "$bak"
  fi
  echo "Installed to $TILES_INSTALL_PATH"
}

case "$BUILD_TYPE" in
  dev)
    cargo build
    cargo install --path . --bins
    install_binary
    ;;
  release)
    cargo build --release
    cargo install --path . --bins --release
    install_binary
    ;;
  check)
    cargo clippy -D warnings
    cargo test
    ;;
  *)
    echo "Usage: $0 [dev|release|check]"
    exit 1
    ;;
esac