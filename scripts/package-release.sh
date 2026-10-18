#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
APP="src-tauri/target/release/bundle/macos/SMB 共享助手.app"
DMG="src-tauri/target/release/bundle/dmg/SMB 共享助手_0.1.0_aarch64.dmg"
[ -d "$APP" ] && [ -f "$DMG" ] || { echo 'Run npm run tauri build first.' >&2; exit 1; }
mkdir -p release
cp "$DMG" release/drag-share-0.1.0-macos-arm64.dmg
ditto -c -k --sequesterRsrc --keepParent "$APP" release/drag-share-0.1.0-macos-arm64.app.zip
(cd release && shasum -a 256 drag-share-0.1.0-macos-arm64.dmg drag-share-0.1.0-macos-arm64.app.zip > SHA256SUMS.txt)
printf 'Packaged release/ artifacts and SHA256SUMS.txt\n'
