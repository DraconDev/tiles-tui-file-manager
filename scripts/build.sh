#!/usr/bin/env bash
set -e

BUILD_TYPE="${1:-dev}"

# Detect where the user's tiles is resolved from PATH
TILES_TARGET=$(command -v tiles 2>/dev/null || echo "")

install_binary() {
  local src="$HOME/.cargo/bin/tiles"
  if [ -n "$TILES_TARGET" ] && [ "$TILES_TARGET" != "$src" ]; then
    local bak="${TILES_TARGET}.bak"
    mv "$TILES_TARGET" "$bak" 2>/dev/null || true
    cp "$src" "$TILES_TARGET" 2>/dev/null || {
      mv "$bak" "$TILES_TARGET" 2>/dev/null || true
      echo "  WARNING: could not install to $TILES_TARGET (binary in use)"
      return
    }
    rm -f "$bak"
    echo "  also installed to $TILES_TARGET"
  fi
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