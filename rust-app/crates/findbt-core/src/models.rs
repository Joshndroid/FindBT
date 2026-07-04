use chrono::{DateTime, Local, NaiveDate};

/// The three-phase capture procedure: capture ambient devices, capture again with the target
/// powered on, then capture once more to see what is still present after the target is powered off.
///
/// Explicit discriminants matter here: `ScanPhase` derives `Ord` and callers rely on
/// `Baseline < Target < Verification` to do "what showed up in an earlier phase" checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScanPhase {
    Baseline = 1,
    Target = 2,
    Verification = 3,
}

impl ScanPhase {
    pub const ALL: [ScanPhase; 3] = [
        ScanPhase::Baseline,
        ScanPhase::Target,
        ScanPhase::Verification,
    ];

    pub fn number(self) -> u8 {
        match self {
            ScanPhase::Baseline => 1,
            ScanPhase::Target => 2,
            ScanPhase::Verification => 3,
        }
    }

    pub fn next(self) -> Option<ScanPhase> {
        match self {
            ScanPhase::Baseline => Some(ScanPhase::Target),
            ScanPhase::Target => Some(ScanPhase::Verification),
            ScanPhase::Verification => None,
        }
    }

    /// Short title used for the sidebar phase tab and the report's phase column.
    pub fn tab_title(self) -> &'static str {
        match self {
            ScanPhase::Baseline => "Baseline Scan",
            ScanPhase::Target => "Target Scan",
            ScanPhase::Verification => "Verification Scan",
        }
    }

    /// Longer label for report tables where a fuller phrase reads better.
    pub fn report_label(self) -> &'static str {
        match self {
            ScanPhase::Baseline => "Phase 1 - Baseline (background before target activation)",
            ScanPhase::Target => "Phase 2 - Target activated",
            ScanPhase::Verification => {
                "Phase 3 - Verification (background after target deactivation)"
            }
        }
    }

    /// Shown under the phase title in the main panel header.
    pub fn description(self) -> &'static str {
        match self {
            ScanPhase::Baseline => {
                "Capture ambient Bluetooth devices in range before the target device is powered on."
            }
            ScanPhase::Target => {
                "Target device is now powered on. Re-scan and compare against the baseline for anything new."
            }
            ScanPhase::Verification => {
                "Repeat the scan once more, unchanged, to see which devices are still present."
            }
        }
    }

    /// Operator instruction used in the wizard/report, not the compact main-screen header
    /// (which uses `description` instead).
    pub fn operator_instruction(self) -> &'static str {
        match self {
            ScanPhase::Baseline => {
                "Instruction: conduct a scan to determine background Bluetooth devices before the target device is activated."
            }
            ScanPhase::Target => {
                "Instruction: manually turn on or activate the target Bluetooth device, wait until it is ready, then start this phase."
            }
            ScanPhase::Verification => {
                "Instruction: turn the target Bluetooth device off, wait until it is off, then conduct the final background Bluetooth scan."
            }
        }
    }
}

/// Kind filter used throughout the UI (`All / BLE / Classic / Unknown` chips) and the device table's
/// kind tag. Each backend maps its platform-specific Bluetooth categories into this small enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceKind {
    Ble,
    Classic,
    Unknown,
}

impl DeviceKind {
    pub const ALL: [DeviceKind; 3] = [DeviceKind::Ble, DeviceKind::Classic, DeviceKind::Unknown];

    pub fn label(self) -> &'static str {
        match self {
            DeviceKind::Ble => "BLE",
            DeviceKind::Classic => "Classic",
            DeviceKind::Unknown => "Unknown",
        }
    }
}

/// Coarse signal-strength bucket. Kept independent of the raw dBm value so the UI's 4-bar
/// indicator and RSSI text color (semantic, not accent-tinted) have one shared source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalStrength {
    Strong,
    Medium,
    Weak,
    Unknown,
}

impl SignalStrength {
    /// Matches the thresholds implied by the design reference's sample data
    /// (roughly: >= -55 dBm strong, >= -70 dBm medium, weaker = weak).
    pub fn from_rssi(rssi: Option<i32>) -> SignalStrength {
        match rssi {
            None => SignalStrength::Unknown,
            Some(v) if v >= -55 => SignalStrength::Strong,
            Some(v) if v >= -70 => SignalStrength::Medium,
            Some(_) => SignalStrength::Weak,
        }
    }

    /// Number of filled bars (0-4) for the 4-bar signal indicator.
    pub fn bars(self) -> u8 {
        match self {
            SignalStrength::Strong => 4,
            SignalStrength::Medium => 3,
            SignalStrength::Weak => 1,
            SignalStrength::Unknown => 0,
        }
    }
}

/// Case metadata captured in the opening wizard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseMetadata {
    pub date: NaiveDate,
    /// Scan reference (shown as "SCAN REFERENCE" in the wizard).
    pub name: String,
    /// Target device reference (shown as "TARGET DEVICE REFERENCE" in the wizard).
    pub section: String,
    /// Operator running the capture.
    pub user: String,
}

impl CaseMetadata {
    pub fn is_complete(&self) -> bool {
        !self.name.trim().is_empty() && !self.section.trim().is_empty()
    }
}

/// The host machine's own Bluetooth radio, detected at wizard time so the operator can confirm
/// (or override) it before scanning starts. `address` is a normalized 12-hex-digit MAC when known.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostAdapterInfo {
    pub name: String,
    pub address: String,
    /// Host machine's name, detected at wizard time and editable by the operator.
    pub computer_name: String,
}

impl HostAdapterInfo {
    pub fn is_detected(&self) -> bool {
        !self.address.trim().is_empty()
    }
}

/// One phase's start/stop record, used for the report's "Phase runs" audit table.
#[derive(Debug, Clone)]
pub struct ScanPhaseRun {
    pub phase: ScanPhase,
    pub started_at: DateTime<Local>,
    pub stopped_at: Option<DateTime<Local>>,
    pub stop_reason: String,
}

impl ScanPhaseRun {
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.stopped_at.map(|stopped| stopped - self.started_at)
    }
}

/// Normalizes free text down to a 12-hex-digit Bluetooth address, or an empty string if the value
/// doesn't reduce to exactly that many hex digits. A free-text radio "tag" (e.g. "Office") can
/// never coincidentally match and mislabel a device.
pub fn normalize_address(value: &str) -> String {
    let hex: String = value.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() == 12 {
        hex.to_uppercase()
    } else {
        String::new()
    }
}
