#!/bin/sh
# Exercise the real backend modules without linking Tauri/GTK or macOS GUI.
set -eu
cd "$(dirname "$0")/.."
ROOT=$PWD
CORE_TEST_DIR=$(mktemp -d)
trap 'rm -rf "$CORE_TEST_DIR"' EXIT HUP INT TERM
node --input-type=module - "$ROOT" "$CORE_TEST_DIR" <<'JS'
import { readFileSync, writeFileSync } from 'node:fs';
const [root, temp] = process.argv.slice(2);
const source = readFileSync(`${root}/src-tauri/Cargo.toml`, 'utf8');
const dependencies = source.split('[dependencies]\n')[1].split('\n[target.')[0]
  .split('\n').filter(line => !line.startsWith('tauri')).join('\n');
writeFileSync(`${temp}/Cargo.toml`, `[package]\nname = "drag-share-core-tests"\nversion = "0.0.0"\nedition = "2021"\nrust-version = "1.92"\n[dependencies]\n${dependencies}\n[dev-dependencies]\ntempfile = "3"\n[lib]\npath = "lib.rs"\n`);
const modules = ['audit', 'awake', 'error', 'model', 'platform', 'process', 'security', 'service'];
writeFileSync(`${temp}/lib.rs`, modules.map(name =>
  `#[path = ${JSON.stringify(`${root}/src-tauri/src/${name === 'platform' ? 'platform/mod' : name}.rs`)}]\nmod ${name};`
).join('\n'));
// Reuse the application's resolved dependency versions as the starting point.
writeFileSync(`${temp}/Cargo.lock`, readFileSync(`${root}/src-tauri/Cargo.lock`));
JS
case "${1:-test}" in
  test) cargo test --manifest-path "$CORE_TEST_DIR/Cargo.toml" --lib ;;
  check-windows) cargo check --manifest-path "$CORE_TEST_DIR/Cargo.toml" --tests --target x86_64-pc-windows-msvc ;;
  *) echo 'Usage: test-rust-core.sh [test|check-windows]' >&2; exit 1 ;;
esac
