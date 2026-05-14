#!/usr/bin/env bash
set -e

BUILD_TYPE="${1:-dev}"

TILES_TARGET=$(command -v tiles 2>/dev/null || echo "")

install_to_path() {
  if [ -n "$TILES_TARGET" ]; then
    local src="$HOME/.cargo/bin/tiles"
    if [ "$TILES_TARGET" = "$src" ]; then
      return
    fi
    local bak="${TILES_TARGET}.bak"
    mv "$TILES_TARGET" "$bak" 2>/dev/null || true
    cp "$src" "$TILES_TARGET" 2>/dev/null || {
      mv "$bak" "$TILES_TARGET" 2>/dev/null || true
      echo "WARNING: could not install to $TILES_TARGET (binary in use)"
      return
    }
    rm -f "$bak"
    echo "Installed to $TILES_TARGET"
  fi
}

case "$BUILD_TYPE" in
  dev)
    cargo build
    cargo install --path . --bins
    install_to_path
    ;;
  release)
    cargo build --release
    cargo install --path . --bins --release
    install_to_path
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