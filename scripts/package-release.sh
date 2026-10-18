#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
APP="src-tauri/target/release/bundle/macos/SMB 共享助手.app"
RELEASE_VERSION=$(node -p "require('./package.json').version")
DMG="src-tauri/target/release/bundle/dmg/SMB 共享助手_${RELEASE_VERSION}_aarch64.dmg"
[ -d "$APP" ] && [ -f "$DMG" ] || { echo 'Run npm run tauri build first.' >&2; exit 1; }
codesign --verify --deep --strict "$APP"
hdiutil verify "$DMG"
mkdir -p release
cp "$DMG" release/drag-share-${RELEASE_VERSION}-macos-arm64.dmg
ditto -c -k --sequesterRsrc --keepParent "$APP" release/drag-share-${RELEASE_VERSION}-macos-arm64.app.zip
(cd release && shasum -a 256 "drag-share-${RELEASE_VERSION}-macos-arm64.dmg" "drag-share-${RELEASE_VERSION}-macos-arm64.app.zip" > SHA256SUMS.txt)
printf 'Packaged release/ artifacts and SHA256SUMS.txt\n'
