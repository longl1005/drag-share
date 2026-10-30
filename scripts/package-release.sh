#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
# No argument preserves the original `npm run tauri build` output layout.
TARGET=${1:-native}
case "$TARGET" in
  native) BUNDLE="${CARGO_TARGET_DIR:-src-tauri/target}/release/bundle" ;;
  aarch64-apple-darwin|x86_64-apple-darwin)
    BUNDLE="${CARGO_TARGET_DIR:-src-tauri/target}/$TARGET/release/bundle" ;;
  *) echo 'Unsupported macOS target.' >&2; exit 1 ;;
esac
if [ "$TARGET" = native ]; then
  case "$(uname -m)" in
    arm64) TARGET=aarch64-apple-darwin ;;
    x86_64) TARGET=x86_64-apple-darwin ;;
    *) echo 'Unsupported host architecture.' >&2; exit 1 ;;
  esac
fi
case "$TARGET" in
  aarch64-apple-darwin) ARCH=arm64; DMG_ARCH=aarch64 ;;
  x86_64-apple-darwin) ARCH=x64; DMG_ARCH=x64 ;;
esac
APP="$BUNDLE/macos/SMB 共享助手.app"
RELEASE_VERSION=$(node -p "require('./package.json').version")
DMG="$BUNDLE/dmg/SMB 共享助手_${RELEASE_VERSION}_${DMG_ARCH}.dmg"
[ -d "$APP" ] && [ -f "$DMG" ] || { echo 'Build app and DMG for the selected target first.' >&2; exit 1; }
# Refuse a stale or incorrectly labelled app before collecting it.
case "$TARGET" in
  aarch64-apple-darwin) EXPECTED=arm64 ;;
  x86_64-apple-darwin) EXPECTED=x86_64 ;;
esac
[ "$(lipo -archs "$APP/Contents/MacOS/drag-share")" = "$EXPECTED" ] || {
  echo "App architecture does not match $TARGET" >&2; exit 1;
}
codesign --verify --deep --strict "$APP"
hdiutil verify "$DMG"
mkdir -p release
cp "$DMG" "release/drag-share-${RELEASE_VERSION}-macos-${ARCH}.dmg"
ditto -c -k --sequesterRsrc --keepParent "$APP" "release/drag-share-${RELEASE_VERSION}-macos-${ARCH}.app.zip"
node scripts/sha256sums.mjs release
printf 'Packaged release/ artifacts and SHA256SUMS.txt\n'
