use std::collections::BTreeMap;

use chrono::{DateTime, Local};

use crate::models::{normalize_address, DeviceKind, ScanPhase, SignalStrength};

/// A single event as reported by a platform Bluetooth backend: "this device was observed (or
/// updated) during this phase, at this moment". `DeviceRegistry::record` folds these into one row
/// per device; `CaptureSession::raw_log` keeps every event exactly as received, for audit.
#[derive(Debug, Clone)]
pub struct RawObservation {
    pub device_id: String,
    pub phase: ScanPhase,
    pub name: String,
    pub address: String,
    pub kind: DeviceKind,
    pub is_paired: bool,
    pub rssi: Option<i32>,
    pub properties_summary: String,
    pub observed_at: DateTime<Local>,
}

/// What the registry remembers about one device in one phase.
#[derive(Debug, Clone)]
pub struct PhaseObservation {
    pub rssi: Option<i32>,
    pub first_seen: DateTime<Local>,
    pub last_seen: DateTime<Local>,
    pub properties_summary: String,
    pub sample_count: u32,
}

impl PhaseObservation {
    pub fn strength(&self) -> SignalStrength {
        SignalStrength::from_rssi(self.rssi)
    }
}

/// One row of the device table: a single device, tracked across all three phases.
#[derive(Debug, Clone)]
pub struct DeviceRecord {
    pub device_id: String,
    pub name: String,
    pub address: String,
    pub kind: DeviceKind,
    pub is_paired: bool,
    pub is_local_radio: bool,
    pub observations: BTreeMap<ScanPhase, PhaseObservation>,
}

impl DeviceRecord {
    /// The earliest phase (Baseline -> Target -> Verification order) this device was seen in.
    pub fn first_seen_phase(&self) -> Option<ScanPhase> {
        self.observations.keys().next().copied()
    }

    /// True when this device was NOT observed in any phase strictly before `phase` - i.e. it is
    /// "new" as of this phase. Phase 2's new-device set driving the operator read of the tool
    /// (ideally exactly one device: the target) is `registry.newly_seen_in(ScanPhase::Target)`.
    pub fn is_new_in(&self, phase: ScanPhase) -> bool {
        self.first_seen_phase() == Some(phase)
    }

    pub fn seen_in(&self, phase: ScanPhase) -> Option<&PhaseObservation> {
        self.observations.get(&phase)
    }
}

/// Live, deduplicated view of every device seen across the whole session: one row per device,
/// each carrying a per-phase observation map. This replaces the older "raw list with duplicate
/// rows per phase" approach entirely - a background device rediscovered in every phase is one
/// row here, not three, and its per-phase presence is a set of markers rather than repeated rows.
#[derive(Debug, Clone, Default)]
pub struct DeviceRegistry {
    devices: BTreeMap<String, DeviceRecord>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn key_for(kind: DeviceKind, device_id: &str) -> String {
        format!("{kind:?}:{device_id}")
    }

    /// Folds one raw observation event into the registry: creates the device's row on first
    /// sight, or updates its per-phase detail (RSSI, last-seen, property summary) on repeat sight.
    pub fn record(&mut self, obs: &RawObservation) {
        let key = Self::key_for(obs.kind, &obs.device_id);
        let entry = self.devices.entry(key).or_insert_with(|| DeviceRecord {
            device_id: obs.device_id.clone(),
            name: obs.name.clone(),
            address: obs.address.clone(),
            kind: obs.kind,
            is_paired: obs.is_paired,
            is_local_radio: false,
            observations: BTreeMap::new(),
        });

        if !obs.name.trim().is_empty() {
            entry.name = obs.name.clone();
        }
        if !obs.address.trim().is_empty() {
            entry.address = obs.address.clone();
        }
        entry.is_paired = obs.is_paired;

        entry
            .observations
            .entry(obs.phase)
            .and_modify(|detail| {
                detail.rssi = obs.rssi.or(detail.rssi);
                detail.last_seen = obs.observed_at;
                detail.properties_summary = obs.properties_summary.clone();
                detail.sample_count += 1;
            })
            .or_insert_with(|| PhaseObservation {
                rssi: obs.rssi,
                first_seen: obs.observed_at,
                last_seen: obs.observed_at,
                properties_summary: obs.properties_summary.clone(),
                sample_count: 1,
            });
    }

    /// Re-evaluates which device (if any) matches the tagged local radio address, so a tag
    /// entered or changed after the scan has started still flags matching rows retroactively.
    pub fn apply_local_radio_tag(&mut self, normalized_address: &str) {
        for device in self.devices.values_mut() {
            device.is_local_radio = !normalized_address.is_empty()
                && normalize_address(&device.address) == normalized_address;
        }
    }

    pub fn devices(&self) -> impl Iterator<Item = &DeviceRecord> {
        self.devices.values()
    }

    pub fn len(&self) -> usize {
        self.devices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    pub fn clear(&mut self) {
        self.devices.clear();
    }

    /// Devices observed at all during `phase` (the raw per-phase count for the report).
    pub fn observed_in(&self, phase: ScanPhase) -> impl Iterator<Item = &DeviceRecord> {
        self.devices
            .values()
            .filter(move |d| d.observations.contains_key(&phase))
    }

    /// Devices whose *first* sighting was in `phase` - the count that matters most for reading
    /// the capture: Phase 2's should ideally be 1 (the target alone), Phase 3's should ideally be
    /// 0. Anything else in Phase 3 means something changed mid-capture; treat the run as unreliable.
    pub fn newly_seen_in(&self, phase: ScanPhase) -> impl Iterator<Item = &DeviceRecord> {
        self.devices.values().filter(move |d| d.is_new_in(phase))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::*;

    fn obs(device_id: &str, phase: ScanPhase, address: &str, rssi: i32) -> RawObservation {
        RawObservation {
            device_id: device_id.to_string(),
            phase,
            name: device_id.to_string(),
            address: address.to_string(),
            kind: DeviceKind::Ble,
            is_paired: false,
            rssi: Some(rssi),
            properties_summary: format!("phase={}", phase.number()),
            observed_at: Local::now(),
        }
    }

    #[test]
    fn registry_deduplicates_device_across_phases() {
        let mut registry = DeviceRegistry::new();

        registry.record(&obs(
            "keyboard",
            ScanPhase::Baseline,
            "AA:AA:AA:AA:AA:AA",
            -62,
        ));
        registry.record(&obs(
            "keyboard",
            ScanPhase::Target,
            "AA:AA:AA:AA:AA:AA",
            -55,
        ));
        registry.record(&obs(
            "keyboard",
            ScanPhase::Verification,
            "AA:AA:AA:AA:AA:AA",
            -58,
        ));

        let devices = registry.devices().collect::<Vec<_>>();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].observations.len(), 3);
        assert!(devices[0].seen_in(ScanPhase::Baseline).is_some());
        assert!(devices[0].seen_in(ScanPhase::Target).is_some());
        assert!(devices[0].seen_in(ScanPhase::Verification).is_some());
    }

    #[test]
    fn newly_seen_in_uses_first_sighting_phase() {
        let mut registry = DeviceRegistry::new();

        registry.record(&obs(
            "background",
            ScanPhase::Baseline,
            "00:00:00:00:00:01",
            -64,
        ));
        registry.record(&obs(
            "background",
            ScanPhase::Target,
            "00:00:00:00:00:01",
            -63,
        ));
        registry.record(&obs(
            "target-only",
            ScanPhase::Target,
            "00:00:00:00:00:02",
            -41,
        ));
        registry.record(&obs(
            "late-change",
            ScanPhase::Verification,
            "00:00:00:00:00:03",
            -72,
        ));

        assert_eq!(registry.newly_seen_in(ScanPhase::Baseline).count(), 1);
        assert_eq!(registry.newly_seen_in(ScanPhase::Target).count(), 1);
        assert_eq!(registry.newly_seen_in(ScanPhase::Verification).count(), 1);

        let target_names = registry
            .newly_seen_in(ScanPhase::Target)
            .map(|device| device.device_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(target_names, vec!["target-only"]);
    }

    #[test]
    fn local_radio_tag_can_be_applied_after_capture() {
        let mut registry = DeviceRegistry::new();
        registry.record(&obs(
            "host-radio",
            ScanPhase::Baseline,
            "AA-BB-CC-DD-EE-FF",
            -35,
        ));

        registry.apply_local_radio_tag("AABBCCDDEEFF");

        assert!(registry.devices().next().unwrap().is_local_radio);
    }
}
