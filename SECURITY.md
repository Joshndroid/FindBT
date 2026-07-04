# Security

## Reporting a vulnerability

Report suspected vulnerabilities privately via GitHub's "Report a vulnerability" (Security → Advisories) on this repository rather than opening a public issue.

## Security posture of the application

FindBT is designed to be easy to reason about in restricted and corporate environments:

- **Offline by construction.** The application makes no network connections of any kind. There is no telemetry, no update checker, no remote logging. The only external process it ever launches is `/usr/sbin/system_profiler` on macOS (fixed absolute path, fixed arguments) as a read-only fallback for adapter detection.
- **Passive observation only.** It never pairs with, connects to, or transmits to discovered devices beyond what the OS's standard discovery/inquiry protocols do. It uses only public OS APIs (CoreBluetooth/IOBluetooth on macOS, WinRT on Windows) with normal user privileges — no elevation, no drivers, no registry writes.
- **Operator-controlled output.** The only file it writes is the report, at a location the operator picks in a native save dialog. Reports are self-contained (no external resources, scripts, or fonts). HTML output escapes all captured values; device names and advertisement data are untrusted input and are treated as such. The PDF writer is an in-repo, dependency-free generator restricted to printable ASCII.
- **Small dependency surface.** Report and session logic (`findbt-core`) depends only on `chrono`. No PDF/HTML third-party libraries. The UI uses egui/eframe and rfd; platform backends use the official `windows` and `objc2-*` bindings.
- **Session data lives in memory.** Nothing is persisted unless the operator exports a report.

## Supply-chain and CI controls

- CI (macOS + Windows): `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test` on every push/PR.
- Pull requests run GitHub dependency review (fails on moderate+ advisories, denies GPL-3.0/AGPL-3.0 licenses).
- Daily scheduled scans: RustSec `cargo audit`, Trivy (vuln/secret/misconfig/license), ClamAV malware scan, actionlint + zizmor workflow linting, and OSSF Scorecard with SARIF upload.
- Dependabot monitors Cargo and GitHub Actions daily.
- Workflow hardening: every job runs `step-security/harden-runner`; the default token permission is `contents: read` with `contents: write` granted only to the release publish job; all checkouts use `persist-credentials: false`. Actions are referenced by version tag and kept current by Dependabot; organizations that require it should additionally pin each action to a full commit SHA.
- Releases: artifacts ship with SHA256 checksum files; macOS supports codesign + notarization via CI secrets; Windows packaging can require a Microsoft Defender scan (`-RequireDefender`) and builds a signed-ready MSI with WiX v4.

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
