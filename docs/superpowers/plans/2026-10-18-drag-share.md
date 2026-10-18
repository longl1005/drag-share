# drag-share implementation plan
Spec: docs/superpowers/specs/2026-10-18-drag-share-design.md
Execution: autonomous inline, authorized by user; no intermediate approval requests.

## Global constraints
Preserve original documents. No real SMB writes during unattended testing. Use typed IPC, least privilege, no credentials/telemetry. Make every failure visible; never substitute preview data into desktop mode.

## Tasks
1. Bootstrap: initialize Git, Vue/Vite/TypeScript/Vitest and Tauri Cargo/config/capabilities. Verify dependency installation and minimal build. Files: package.json, tsconfig*, vite.config.ts, src-tauri/{Cargo.toml,build.rs,tauri.conf.json}.
2. Domain red/green: tests before implementation of errors, ShareItem, sharing parser, disabled state parser, shell/AppleScript escaping, URL encoding, validation, process timeout and audit rotation. Files: src-tauri/src/{model,error,security,process,audit}.rs and platform/{mod,macos,unsupported}.rs. Prefer realistic fixtures, including macOS JSON/text variations and malicious names.
3. Native integration: trait-backed serialized service, verify writes, cancellation, networking and audit. Typed IPC for snapshot/toggle/add/remove/copy/reveal/settings/autostart/hide/dialog; menu and shortcut use same service. Native tray state and monitor-aware placement. Files: service.rs, commands.rs, desktop.rs, lib.rs, main.rs, Info.plist.
4. UI red/green: typed models/bridge/controller, prototype-equivalent panel components, status/empty/loading/unknown/drop/errors, menu/dialog accessibility and dark mode. Files: src/{App.vue,main.ts,style.css,types.ts,bridge.ts,useShare.ts}, components and tests. Isolate explicit browser preview.
5. Verification: npm test/build; cargo fmt/test/clippy; native release build; browser visual checks. Fix concrete failures with regression tests. Inspect bundled executable, app metadata, entitlements and artifact size. Native read-only smoke mode must not register autostart or mutate SMB.
6. Delivery: README, manual acceptance checklist, RTM, verification report, local commits, .app and DMG in release/. No push/publish/install. Distinguish tested behavior from manual pending checks.

## Shared interfaces
Snapshot {status: enabled|disabled|unknown, shares, address, addressFallback, warnings}; ShareItem {id, name, path, readOnly, guestAccess, exists}; typed Error {code,message}; action outcome canceled represented separately. Native authoritative mutation guard shared by all entry points. UI reloads snapshot after actions and on panel-open; latest refresh wins.
