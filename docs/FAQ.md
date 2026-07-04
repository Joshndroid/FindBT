# FindBT FAQ — common issues and fixes

Grouped by where they occur. If an issue is not covered here, capture the exact status-bar message (bottom-left of the sidebar) — every backend error surfaces there.

## Starting the app

**macOS: "FindBT can't be opened because it is from an unidentified developer."**
The build is unsigned or not notarized. Right-click `FindBT.app` → Open → Open. If it was quarantined after download: `xattr -cr /path/to/FindBT.app`. For distribution, sign and notarize via the Release workflow secrets.

**macOS: app opens but never shows any BLE device, or a row says "CoreBluetooth central is not powered on".**
Three causes, in order of likelihood: Bluetooth is off (turn it on in Control Centre); Bluetooth permission was denied — System Settings → Privacy & Security → Bluetooth → enable FindBT; or you ran the bare `findbt-app` binary instead of `FindBT.app`. The permission prompt only works from the bundled app because it carries the Bluetooth usage strings in `Info.plist`. Build the bundle with `scripts/build-macos-app.sh`.

**Windows: SmartScreen says "Windows protected your PC".**
Unsigned binary. Click More info → Run anyway, or have IT sign the executable / add it to the allowlist. Verify the SHA256 against the shipped `.sha256.txt` first.

**Windows: app starts but no devices ever appear.**
Check that Bluetooth is on (Settings → Bluetooth & devices), airplane mode is off, and the machine actually has a Bluetooth adapter (Device Manager → Bluetooth). On desktops without built-in Bluetooth you need a USB adapter. If the adapter exists but behaves oddly, update the Bluetooth driver — WinRT watcher behavior is driver-dependent.

## The wizard

**"Begin Scan" stays disabled.**
Name and Section are required, and the Case date must parse as `YYYY-MM-DD`. Fill all three.

**Host adapter fields are empty (adapter not detected).**
Detection can fail on machines without Bluetooth or with restricted drivers. Type the adapter's address manually (macOS: System Information → Bluetooth; Windows: Device Manager → Bluetooth adapter → Advanced) or enter a short text tag. Scanning still works; the tag only controls how the computer's own radio is labeled in results.

## During a scan

**Status bar: "Scan could not start: a scan is already running."**
A phase is still active. Click `Stop Scan` first, then start the next phase.

**Status bar: "IOBluetooth inquiry could not start: IOReturn=..." (macOS).**
The Classic-Bluetooth inquiry couldn't start, usually because Bluetooth is powered off or the radio is busy. Toggle Bluetooth off/on and retry. BLE scanning may still be running when this appears — check the raw log.

**A device I can see on my phone doesn't appear.**
FindBT only hears devices that are transmitting and discoverable during the scan window. Paired-and-idle devices, devices connected elsewhere, and devices with discovery off can be silent. Lengthen the scan window and move closer. Absence from the report is not proof of absence.

**The same device shows up as several rows.**
Expected in two cases: a dual-mode device appears once as BLE and once as Classic (different transports, different identifiers), and modern phones/wearables rotate random BLE addresses every few minutes, creating a new row per rotation. Note it in the record; it is a property of Bluetooth privacy, not a defect.

**BLE rows on macOS have no address.**
Apple's CoreBluetooth API never exposes BLE MAC addresses to apps. Rows are keyed by a CoreBluetooth UUID that is stable on that machine only. Use Windows if raw BLE addresses are required.

**RSSI shows "unknown" / "not seen" for Classic devices on macOS.**
IOBluetooth only reports RSSI for connected Classic devices. Unconnected Classic discovery has no signal reading; this is an OS limitation.

**RSSI values jump around.**
Normal radio behavior — reflections, bodies moving, antenna orientation. Judge by the strength bucket (strong/medium/weak) over the window, not single readings.

**Phase 2 shows many "newly seen" devices, not one.**
The environment changed between phases (people/devices arrived) or the baseline was too short to catch slow advertisers. Re-run with a longer baseline and the environment as still as possible; document anything you can't control.

**Phase 3 shows a newly seen device.**
Something entered the environment mid-capture. The run should be treated as unreliable per procedure — repeat the capture if possible, otherwise document the anomaly.

## Reports

**"Report save failed: permission denied" (or similar).**
The chosen folder isn't writable (network share, protected directory). Save somewhere writable and copy afterwards.

**The PDF shows `?` where the device name has special characters.**
The built-in PDF writer is ASCII-only by design (no embedded fonts); non-ASCII characters become `?`. Export HTML for full Unicode fidelity — switch format in Settings → Report generation.

**The report's timestamps look wrong.**
All timestamps come from the scanning computer's clock, recorded with UTC offset. If the clock or timezone was wrong during capture, it is wrong in the report — this is why the procedure says to verify the clock first.

**I generated the wrong format.**
Settings → Report generation → pick the other format → `Generate Report` again. Both formats contain identical content; generating both is fine.

## Building and packaging

**`cargo: command not found`.**
Install Rust via rustup.rs, then restart the terminal.

**Windows build warning: "rc.exe was not available; FindBT.exe will use the default executable icon."**
Harmless — the icon resource compiler ships with Visual Studio / Windows SDK. Install the Windows SDK or ignore it; only the exe icon is affected.

**`package-windows.ps1` doesn't produce an MSI.**
WiX v4 isn't installed. `dotnet tool install --global wix --version 4.*`, then re-run. Portable zips are produced regardless; use `-RequireInstaller` to make the MSI mandatory.

**`package-windows.ps1 -RequireDefender` fails.**
Microsoft Defender's `MpCmdRun.exe` wasn't found or the scan flagged something. Check the script output; without `-RequireDefender` the scan is skipped when unavailable.

**CI clippy failures after editing.**
CI runs `cargo clippy --workspace --all-targets -- -D warnings` — every warning is an error. Run the same command locally before pushing, plus `cargo fmt --all`.
