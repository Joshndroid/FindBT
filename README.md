# FindBT

FindBT is an offline Bluetooth capture tool for macOS and Windows. It records Bluetooth observations from the local radio, guides an operator through a three-phase capture workflow, and exports a standalone report as HTML or PDF (selectable in Settings).

FindBT deliberately does not infer, identify, rank, or suggest which observed Bluetooth device is the target device. It records what the operating system's Bluetooth stack reported, when it reported it, and in which phase — nothing more. Interpretation stays with the operator.

## The three-phase method

The capture procedure isolates a target device by differencing three scans of the same environment:

1. **Baseline** — scan with the target device powered off. Everything seen here is background.
2. **Target** — power the target device on, scan again. A device that appears now, but not in the baseline, is a candidate.
3. **Verification** — power the target device off, scan once more. A candidate that disappears again corroborates the phase-2 observation; a device that first appears now means the environment changed mid-capture and the run should be treated as unreliable.

The report's "Newly seen" column encodes exactly this: the first phase in which each device was observed. Ideally phase 2 has exactly one newly seen device (the target) and phase 3 has zero.

## How each platform obtains its results

All backends implement one Rust trait (`BluetoothBackend` in `crates/findbt-backend`) and stream `RawObservation` events into the shared session/registry/report logic in `crates/findbt-core`. The UI is identical on both platforms.

### macOS (program: `FindBT.app`, binary `FindBT` built from `findbt-app`)

The macOS backend (`crates/findbt-backend/src/macos.rs`) combines two native Apple frameworks plus one system utility:

- **CoreBluetooth (BLE discovery).** The app creates a `CBCentralManager` with a delegate and calls `scanForPeripheralsWithServices:options:` once the radio reports `poweredOn`. Every advertisement Apple delivers arrives through the `centralManager:didDiscoverPeripheral:advertisementData:RSSI:` delegate callback and is recorded with its advertised local name, RSSI, and connectable flag. CoreBluetooth intentionally never exposes BLE MAC addresses to applications, so BLE rows are keyed by the peripheral's CoreBluetooth UUID (`corebluetooth:<uuid>`); the UUID is stable per host machine but is not the device's radio address.
- **IOBluetooth (Classic discovery).** The app runs an `IOBluetoothDeviceInquiry` with `setSearchType(Classic)` and a 10-second inquiry length that automatically restarts on completion, so scanning is continuous until the operator stops the phase. Devices arrive via `deviceInquiryDeviceFound:` / `deviceInquiryDeviceNameUpdated:` callbacks with their real MAC address and pairing state. At scan start the backend also enumerates `IOBluetoothDevice.pairedDevices()` so paired-but-quiet Classic devices are captured. RSSI is only available for connected Classic devices (`rawRSSI`).
- **Adapter identification.** The host adapter shown in the wizard comes from `IOBluetoothHostController.defaultController` (name + address); if that fails, the backend runs `/usr/sbin/system_profiler SPBluetoothDataType -json` and parses `controller_properties` as a fallback.

Because CoreBluetooth is privacy-gated, macOS shows a Bluetooth permission prompt the first time — this requires running the bundled `FindBT.app` (its `Info.plist` carries `NSBluetoothAlwaysUsageDescription`), not the bare binary.

### Windows (program: `FindBT.exe`)

The Windows backend (`crates/findbt-backend/src/windows.rs`) uses WinRT APIs from the `windows` crate:

- **Device watchers (BLE + Classic enumeration).** Four `DeviceInformation` `DeviceWatcher`s run concurrently, built from `BluetoothLEDevice.GetDeviceSelectorFromPairingState(...)` and `BluetoothDevice.GetDeviceSelectorFromPairingState(...)` for both paired and unpaired states, with `DeviceInformationKind.AssociationEndpoint`. Each watcher requests the `System.Devices.Aep.DeviceAddress`, `IsPaired`, `Le.IsConnectable`, and `SignalStrength` properties, and every `Added`/`Updated` event becomes an observation.
- **BLE advertisement watcher.** A `BluetoothLEAdvertisementWatcher` in `Active` scanning mode reports every advertisement packet with the advertiser's address, local name, and raw RSSI in dBm — this is the fastest, most complete BLE source on Windows.
- **Adapter identification.** `BluetoothAdapter.GetDefaultAsync()` supplies the host radio's address and device id for the wizard.

### Mock backend (development)

Building with `--features mock-backend` swaps in a deterministic scripted backend (`mock.rs`): a fixed cast of devices per phase (background phone/keyboard, a "Target Speaker" that only appears in phase 2, a "Door Sensor" that only appears in phase 3) with slowly varying RSSI. It exists so the UI and both report formats can be exercised and verified on any machine without Bluetooth hardware. On Linux the mock backend is used automatically.

## Report formats

`Generate Report` exports whichever format is selected under **Settings → Report generation**:

- **HTML** (`findbt-core/src/report.rs`) — a standalone, dependency-free HTML page; all values HTML-escaped.
- **PDF** (`findbt-core/src/pdf.rs`) — an A4-landscape PDF written by a small built-in writer using only standard PDF Type1 fonts; no third-party PDF library and no network access. Non-ASCII characters are rendered as `?` (use HTML if device names need full Unicode).

Both formats contain the same sections: capture metadata, phase runs, per-phase summary, deduplicated device registry, and the append-only raw audit log. Operating caveats (system clock, radio/antenna limits) are covered in [QUICKSTART.md](QUICKSTART.md) rather than printed on the report.

## Development

VS Code defaults target the Rust app. `Terminal > Run Build Task` builds the native debug app for the current OS, and `F5` launches `FindBT Rust (native debug)` through CodeLLDB. Use `FindBT Rust (mock debug)` for UI work without Bluetooth hardware.

Useful commands:

```bash
cd rust-app
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p findbt-app
cargo run -p findbt-app --features mock-backend
```

## Packaging

macOS:

```bash
cd rust-app
scripts/package-macos.sh
```

This creates a `.pkg` installer, portable zip, offline portable zip, SHA256 files, and `local-release.txt` under `rust-app/dist/macos/artifacts/`.

Windows:

```powershell
cd rust-app
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-windows.ps1 -RequireDefender
```

This creates portable/offline zips, SHA256 files, and `local-release.txt` under `rust-app/dist/windows/artifacts/`. It also creates an MSI when WiX v4 is installed locally and runs Microsoft Defender scans when available.

Portable archives include the app, `quickstart.txt`, and `local-release.txt`.

## GitHub Automation

This repository includes GitHub automation for Dependabot, CI, dependency review, daily security scanning, and release packaging.

- Dependabot checks Cargo and GitHub Actions daily.
- CI builds/checks/tests the Rust app on macOS and Windows.
- Pull requests run dependency review (fails on moderate+ vulnerabilities and GPL/AGPL licenses).
- Daily security scans run RustSec cargo audit, Trivy, ClamAV, actionlint, zizmor workflow analysis, and OSSF Scorecard.
- The manual `Release` workflow supports dry runs, macOS signing/notarization hooks, WiX MSI creation, and Windows Defender scans.
- Workflow tokens are least-privilege (`contents: read` by default, `contents: write` only on the release publish job) and checkouts use `persist-credentials: false`.

Run the `Release` workflow with `dry_run: true` to build and upload release artifacts without publishing a GitHub release.

## Operator Notes

FindBT does not infer, identify, rank, or suggest which observed Bluetooth device is the target device. It lists observed names, addresses when the OS exposes them, device IDs, signal strength when available, timestamps, and phase labels.

For repeatable captures, document the host machine, Bluetooth adapter, driver version, antenna position, phase timing, and physical environment alongside the exported report.
