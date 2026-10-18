# drag-share implementation design

Approved scope: user confirmed P0 + P1, macOS first, unattended testing on 2026-10-18. PRD.md is the functional authority; prototype.html is the visual authority. This authorization includes autonomous planning and implementation without further review gates.

## Product and boundaries
A Chinese macOS 13+ tray utility for local SMB sharing. Tauri 2, Vue 3, TypeScript, Rust 1.92+. Windows/Linux expose explicit unsupported adapters. No telemetry, credentials, recursive file scanning, publishing, or automatic login-item enablement. Deliver source, tests, native Apple Silicon app and attempt DMG. Signing/notarization is outside scope.

## Architecture
Vue renders a 320×440 panel, status banner, share list, empty/loading/error states and drop target. A composable owns serialized actions and refresh. Native Rust owns authoritative service state, share validation, tray, window placement, system dialogs, clipboard, shortcut and autostart. IPC exposes typed operations only, no arbitrary process or shell endpoint. Rust SmbPlatform isolates platform behavior. Read commands use bounded subprocess execution; write commands use escaped fixed AppleScript administrator commands. Shared backend lock serializes writes from UI and tray. While native dialogs/authorization are active, suppress blur hiding.

## Native semantics
Service enable/disable never change share records. Use launchctl disabled override plus loaded job checks, not daemon PID alone (socket activation). Enable handles already loaded services, verifies TCP port and retries kickstart once; disable tolerates already unloaded job but must verify disabled override. Share removal resolves an existing record by stable record name; sharing -r accepts record name, not path. Parse SMB fields separately from record name and exclude non-SMB records. Verify add/remove after commands. Canonicalize directory paths and reject duplicates before elevation; remove stale records without requiring the physical path to exist. New records inherit default system permissions and expose actual reported attributes, never fabricate ACLs.

## UX
Match prototype colors, typography, dimensions and light/dark adaptation; omit prototype stage and state switches. Tray starts hidden without Dock, toggles on left click, native menu on right. Position in monitor work area, clamp with 8px inset. Refresh on reveal, never poll idle. Esc/blur hides except pending native operations. Stop sharing requires confirmation and never deletes directory contents. Canceled elevation is silent. Errors distinguish unknown status from disabled. Busy state disables conflicting actions. Shortcut Ctrl+Option+S, login launch stays hidden. No production mock data; optional explicit browser preview has isolated fixture data and cannot invoke native commands.

## Verification and delivery
Rust tests exercise command escaping, parsing, status detection, address encoding, directory validation, cancellation/timeouts, state invariants and log rotation. Vue tests cover disabled/busy/unknown behavior, cancellation and errors. Build/typecheck, formatting and lint, browser screenshots and native read-only smoke checks. No elevated calls, SMB mutations, or login-item changes in unattended verification. Record admin, real device, reboot, multi-monitor and one-hour memory validation as manual. Logs rotate 5MiB, 3 historical files, local only. Include RTM and honest verification report.
