#!/usr/bin/env bash
set -e

BUILD_TYPE="${1:-dev}"

case "$BUILD_TYPE" in
  dev)
    cargo build
    cargo install --path . --bins
    ;;
  release)
    cargo build --release
    cargo install --path . --bins --release
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