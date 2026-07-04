# FindBT Quickstart

This document is the operator hand-off guide for FindBT. It explains what the program does, how to run a capture correctly, how to verify the result, and how to explain the process to someone else. It assumes no prior knowledge of the tool. Common problems are covered in [docs/FAQ.md](docs/FAQ.md).

## 1. What FindBT is

FindBT is an offline desktop tool (macOS: `FindBT.app`, Windows: `FindBT.exe`) that listens to the computer's own Bluetooth radio and records every device the operating system reports, across a three-phase procedure. At the end it exports a standalone report (HTML or PDF, chosen in Settings) containing everything it heard, when, and in which phase.

What it does not do, by design:

- It does not decide, rank, or suggest which device is the target. The report presents observations; the operator interprets them.
- It does not pair with or connect to any device, install drivers, use the internet, or write anywhere except the report file the operator chooses to save.

## 2. The idea behind the three phases

You isolate one device by scanning the same environment three times and comparing:

- **Phase 1 — Baseline.** The target device is OFF. Everything heard now is background (other people's phones, keyboards, TVs, and so on).
- **Phase 2 — Target.** The target device is turned ON and scanned. Whatever appears now that was not in the baseline is a candidate for the target.
- **Phase 3 — Verification.** The target device is turned OFF again and the environment is scanned once more. The candidate should disappear; the background should still be there.

A clean run reads like this in the report's Phase summary table: Phase 2 has exactly one "Newly seen" device, and Phase 3 has zero. If Phase 3 shows newly seen devices, the environment changed during the capture (someone walked in with a phone, a device woke up) and the run should be repeated or the anomaly documented.

## 3. Getting the program running

### macOS

1. Preferred: install from `FindBT-vX.Y.Z-macos-installer.pkg`, or unzip the portable zip and keep `FindBT.app` intact.
2. Verify the download: each artifact ships with a `.sha256.txt` file — compare with `shasum -a 256 <file>`.
3. Launch `FindBT.app`. On first launch macOS asks for Bluetooth permission — click Allow. (This prompt only appears for the `.app` bundle; do not run the bare binary.)
4. If Gatekeeper blocks an unsigned build: right-click the app, choose Open, then Open again.

### Windows

1. Preferred: install the MSI, or unzip the portable zip to any folder and run `FindBT.exe` from there.
2. Verify the download against the accompanying `.sha256.txt` file (`Get-FileHash <file> -Algorithm SHA256` in PowerShell).
3. If SmartScreen warns about an unrecognized app: More info → Run anyway (or have IT sign/whitelist the binary).
4. Make sure Bluetooth is turned on in Windows Settings and the machine is not in airplane mode.

### Building from source (developers)

Install Rust (rustup.rs), then:

```bash
cd rust-app
cargo run -p findbt-app                     # native backend for this OS
cargo run -p findbt-app --features mock-backend   # simulated devices, no hardware needed
```

## 4. Before you start: preparation checklist

- Check the computer's date and time. Every timestamp in the report comes from this machine's system clock; if the clock is wrong, the whole timeline is wrong. Fix it first.
- Have the target device at hand, powered OFF, and know how to power it on and off quickly.
- Keep the scanning computer stationary for the whole capture.
- Note for the record: scan location, host machine, Bluetooth adapter make/model, driver version (Windows), and antenna position.

## 5. Running a capture, step by step

1. **Launch FindBT.** The session wizard appears.
2. **Fill in the case fields:** Case date (prefilled, `YYYY-MM-DD`), Name (person, device owner, or case name), and Section (team, section, or exhibit reference). Name and Section are required before the button enables.
3. **Confirm the host adapter.** FindBT auto-detects the computer's own Bluetooth radio (name and address) and shows it in the wizard. If detection failed or you want a custom label, type the radio's address or a short tag yourself. This "tagged local radio" is used to mark the computer's own radio in the results so nobody mistakes it for a discovered device.
4. **Click `Begin Scan`.** The main screen opens with the three phases listed in the sidebar: 1 Baseline Scan, 2 Target Scan, 3 Verification Scan.
5. **Phase 1 — Baseline.** Target device is OFF. Select the Baseline phase, read the instruction under the heading, click `Start Scan`. Let it run (2–5 minutes is typical; longer catches slow advertisers). Devices appear live in the table. Click `Stop Scan` when done. Phases do not stop by themselves — the operator controls the window.
6. **Phase 2 — Target.** Click phase 2 in the sidebar. Power the target device ON and wait until it is fully started. Click `Start Scan`, run a comparable window, click `Stop Scan`. Watch the phase-marker columns: a device with a filled marker in Target but not Baseline is what you are looking for.
7. **Phase 3 — Verification.** Click phase 3. Power the target device OFF and wait until it is fully off. Click `Start Scan`, run a comparable window, `Stop Scan`.
8. **Choose the report format** (once, or whenever you want to change it): click `Settings` in the title bar and pick HTML export or PDF export under "Report generation".
9. **Click `Generate Report`,** choose where to save the file, and store it with the case material. Generate both formats if useful — the content is identical.

Useful controls during a capture: the filter box narrows the table by name/address/id; the All/BLE/Classic/Unknown chips filter by device kind; `Rescan` re-runs a phase you have already run (all runs are kept and listed in the report's Phase runs table); `Reset capture` clears everything and starts the session's capture data over.

## 6. Reading the screen and the report

Each table row is one device, tracked across all three phases. The three marker columns (Baseline / Target / Verification) mean: filled circle = seen in that phase; hollow circle = that phase ran but the device was not seen; faint circle = that phase has not run yet. RSSI is the signal strength in dBm for the currently selected phase (closer to 0 = stronger; -42 is strong, -75 is weak; strong usually means physically near).

The exported report contains, in order: capture metadata (including the tagged local radio), Phase runs (every start/stop with timestamps, duration, and stop reason), Phase summary (per phase: raw observation count, unique addresses, and newly-seen count), Device registry (one row per device with per-phase RSSI), and the Raw audit log (every single backend event, unfiltered, append-only).

Platform notes for reading results:

- On macOS, BLE devices show no MAC address — Apple's CoreBluetooth API hides it — so BLE rows are keyed by a per-machine CoreBluetooth UUID instead. Classic Bluetooth rows do show real addresses.
- On Windows, both the WinRT device watchers and the BLE advertisement watcher report addresses.
- Many modern devices (phones especially) rotate random BLE addresses every few minutes. The same physical device can therefore appear as several rows. This is a property of Bluetooth privacy, not a tool error.

## 7. Verifying the tool works (before field use)

- **Without hardware:** build and run with `--features mock-backend`. The mock scripts a known cast: "Target Speaker" appears only in phase 2 and "Door Sensor" only in phase 3. Run all three phases and confirm the report's Newly seen counts read 3 / 1 / 1 for phases 1/2/3 and that the registry markers match the script. This proves the phase logic, registry, and both report formats end-to-end.
- **With hardware:** use any Bluetooth device you control (a speaker or earbuds) as a stand-in target and run the full procedure. Confirm it is absent from phase 1, newly seen in phase 2, and absent again in phase 3.
- **Automated tests:** `cargo test --workspace` covers the registry dedup/newly-seen logic and both report generators.

## 8. Explaining the process (for statements or review)

A capture can be described accurately like this: "The computer's built-in Bluetooth radio was used to record all discoverable Bluetooth activity in three windows: before the device of interest was powered on, while it was on, and after it was powered off. The software records what the operating system's Bluetooth stack reports — device names, addresses where the OS exposes them, signal strength, and timestamps from the computer's clock — and marks in which window each device was first seen. It performs no interpretation; the appearance of a device only in the middle window, corroborated by its disappearance in the third, was assessed by the operator."

Honest limitations to state alongside results: detection depends on the target actually transmitting and being discoverable; range, antenna, interference, and driver behavior limit what is heard; absence from the report is not proof a device was absent; BLE address rotation can split one device across rows; and all timestamps depend on the scanning computer's clock, which was checked before capture.

## 9. If something goes wrong

See [docs/FAQ.md](docs/FAQ.md) for common issues: permission prompts, Gatekeeper/SmartScreen, "CoreBluetooth central is not powered on", adapters not detected, missing RSSI values, duplicate rows, and report-saving errors.
