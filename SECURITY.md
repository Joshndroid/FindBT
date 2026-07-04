# Security

## Reporting a vulnerability

Report suspected vulnerabilities privately via GitHub's "Report a vulnerability" (Security → Advisories) on this repository rather than opening a public issue.

## Security posture of the application

FindBT is designed to be easy to reason about in restricted and corporate environments:

- **Offline by construction.** The application makes no network connections of any kind. There is no telemetry, no update checker, no remote logging. The only external process it ever launches is `/usr/sbin/system_profiler` on macOS (fixed absolute path, fixed arguments) as a read-only fallback for adapter detection.
- **Passive observation only.** It never pairs with, connects to, or transmits to discovered devices beyond what the OS's standard discovery/inquiry protocols do. It uses only public OS APIs (CoreBluetooth/IOBluetooth on macOS, WinRT on Windows) with normal user privileges — no elevation, no drivers. The only registry access is the installed Windows build storing its own UI preferences under `HKCU\Software\FindBT` (see below); portable builds never touch the registry.
- **Operator-controlled output.** Beyond the settings store described below, the only file it writes is the report, at a location the operator picks in a native save dialog. Reports are self-contained (no external resources, scripts, or fonts). HTML output escapes all captured values; device names and advertisement data are untrusted input and are treated as such. The PDF writer is an in-repo, dependency-free generator restricted to printable ASCII.
- **Small dependency surface.** Report and session logic (`findbt-core`) depends only on `chrono`. No PDF/HTML third-party libraries. The UI uses egui/eframe and rfd, with `serde`/`serde_json` for settings and `winreg` (Windows only) for the installed build's preference store; platform backends use the official `windows` and `objc2-*` bindings.
- **Capture data lives in memory.** Session and capture data are never persisted unless the operator exports a report. The only thing persisted is the UI preference store described next.

## Settings persistence and hardening

FindBT stores exactly one category of data between runs: UI preferences (currently the theme choice). Where it lives depends on the deployment, detected at runtime by whether the app's own directory is writable:

- **Portable zips:** `settings.json` beside the executable — the portable copy is fully self-contained and leaves no registry keys or profile files on the host.
- **Installed via MSI (Windows):** per-user values under `HKCU\Software\FindBT`. No settings.json is created.
- **Installed via pkg (macOS):** `~/Library/Application Support/FindBT/settings.json` (macOS has no registry; this is the platform-standard location).

The settings source is treated as untrusted input and is locked to this app. A `settings.json` is honored only if all of the following hold: it is no larger than 8 KB, parses as JSON matching the exact expected schema with unknown fields rejected, carries the `"app": "FindBT"` marker and a supported `settings_version`, and every value belongs to a closed enum — free-form strings are never trusted. Registry values are validated identically. Any file or key that fails any check — missing, corrupt, oversized, wrong app, wrong version, unexpected value — is ignored and the app runs on built-in defaults; a hostile or damaged settings source cannot alter behavior beyond the fixed preference set and can never prevent the app from starting. Writes are atomic (temp file + rename), so interrupted writes cannot leave a truncated file to be parsed later.

## Supply-chain and CI controls

- CI (macOS + Windows): `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test` on every push/PR.
- Code scanning: GitHub's built-in CodeQL default setup (Rust), Dependabot alerts (covers RustSec advisories), and secret scanning, enabled in repository settings.
- Pull requests run GitHub dependency review (fails on moderate+ advisories).
- Pull requests that touch `.github/workflows/` run actionlint and zizmor workflow security analysis with SARIF upload.
- Dependabot monitors Cargo and GitHub Actions daily.
- Workflow hardening: every job runs `step-security/harden-runner`; the default token permission is `contents: read` with `contents: write` granted only to the release publish job; all checkouts use `persist-credentials: false`; every action is pinned to a full commit SHA (with the version noted in a comment) and kept current by Dependabot.
- Releases: three isolated build jobs (macOS installer, Windows installer, Windows offline portable — the offline portable is built on its own runner and uploaded as its own artifact so it can be audited as a clearly separated distribution). Artifacts ship with SHA256 checksum files; macOS supports codesign + notarization via CI secrets; Windows packaging can require a Microsoft Defender scan (`-RequireDefender`) and builds a signed-ready MSI with WiX v4.

## Deploying in a corporate environment

- Prefer the signed installer paths: notarized `.pkg` on macOS, signed MSI on Windows (signing identities/secrets are hooks in the Release workflow).
- Verify artifact SHA256 checksums on receipt; distribute checksums out of band if required.
- The app needs Bluetooth permission on macOS (per-user privacy prompt) and an enabled Bluetooth adapter on Windows; no other entitlements, firewall rules, or proxy exceptions are needed — it should be expected to make zero network calls, which makes egress-monitoring exceptions unnecessary.
- Treat exported reports as potentially sensitive: they enumerate nearby device names/addresses observed at a time and place. Store them with the same handling as the underlying case material.
- The portable zips run without installation; if application allowlisting (AppLocker/Santa) is in use, allow the specific hash you validated.

## Known, accepted limitations

- Bluetooth capture inherently records third-party device identifiers from the surrounding environment; minimize scan windows and handle reports accordingly.
- BLE rows on macOS are keyed by CoreBluetooth UUIDs (Apple hides BLE MACs); identifiers in reports are therefore partly host-specific.
- Timestamps derive from the host clock; the operating procedure (QUICKSTART) requires a clock check before capture.
