#!/bin/sh
# Explicit targets keep packaging independent of the host CPU.
set -eu
cd "$(dirname "$0")/.."
TARGET=${1:-$(uname -m)-apple-darwin}
case "$TARGET" in
  arm64-apple-darwin) TARGET=aarch64-apple-darwin ;;
  aarch64-apple-darwin|x86_64-apple-darwin) ;;
  *) echo 'Usage: scripts/build-macos.sh [aarch64-apple-darwin|x86_64-apple-darwin]' >&2; exit 1 ;;
esac
[ "$(uname -s)" = Darwin ] || { echo 'macOS with Xcode Command Line Tools is required.' >&2; exit 1; }
if [ -n "${CARGO_TARGET_DIR:-}" ]; then
  case "$CARGO_TARGET_DIR" in
    /*) ;;
    *) CARGO_TARGET_DIR="$PWD/$CARGO_TARGET_DIR" ;;
  esac
  export CARGO_TARGET_DIR
fi
export MACOSX_DEPLOYMENT_TARGET=13.0
rustup target add "$TARGET"
npm run tauri build -- --target "$TARGET" --bundles app,dmg --ci -- --locked
scripts/package-release.sh "$TARGET"
