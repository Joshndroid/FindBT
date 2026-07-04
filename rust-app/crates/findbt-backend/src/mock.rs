use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use chrono::Local;
use findbt_core::{DeviceKind, HostAdapterInfo, RawObservation, ScanPhase};

use crate::{BackendError, BluetoothBackend};

pub struct MockBluetoothBackend {
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Default for MockBluetoothBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBluetoothBackend {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            worker: None,
        }
    }
}

impl BluetoothBackend for MockBluetoothBackend {
    fn default_adapter(&self) -> Option<HostAdapterInfo> {
        Some(HostAdapterInfo {
            name: "Mock Bluetooth Adapter".to_string(),
            address: "02:00:5E:00:53:01".to_string(),
        })
    }

    fn start(&mut self, tx: Sender<RawObservation>, phase: ScanPhase) -> Result<(), BackendError> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(BackendError::AlreadyRunning);
        }

        let running = Arc::clone(&self.running);
        self.worker = Some(thread::spawn(move || {
            let script = scripted_observations(phase);
            let mut tick = 0;
            while running.load(Ordering::SeqCst) {
                for template in &script {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }
                    let mut observation = template.clone();
                    observation.rssi = observation.rssi.map(|rssi| rssi - (tick % 4));
                    observation.observed_at = Local::now();
                    if tx.send(observation).is_err() {
                        running.store(false, Ordering::SeqCst);
                        return;
                    }
                    thread::sleep(Duration::from_millis(180));
                }
                tick += 1;
            }
        }));

        Ok(())
    }

    fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for MockBluetoothBackend {
    fn drop(&mut self) {
        self.stop();
    }
}

fn scripted_observations(phase: ScanPhase) -> Vec<RawObservation> {
    let mut devices = vec![raw(
        "mock-phone",
        phase,
        "Nearby Phone",
        "10:20:30:40:50:60",
        DeviceKind::Ble,
        false,
        -61,
        "connectable=true source=mock",
    )];

    match phase {
        ScanPhase::Baseline => {
            devices.push(raw(
                "mock-keyboard",
                phase,
                "Desk Keyboard",
                "20:20:30:40:50:60",
                DeviceKind::Classic,
                true,
                -48,
                "paired=true source=mock",
            ));
            devices.push(raw(
                "mock-host",
                phase,
                "Mock Bluetooth Adapter",
                "02:00:5E:00:53:01",
                DeviceKind::Unknown,
                true,
                -30,
                "local=true source=mock",
            ));
        }
        ScanPhase::Target => {
            devices.push(raw(
                "mock-keyboard",
                phase,
                "Desk Keyboard",
                "20:20:30:40:50:60",
                DeviceKind::Classic,
                true,
                -50,
                "paired=true source=mock",
            ));
            devices.push(raw(
                "mock-target-speaker",
                phase,
                "Target Speaker",
                "30:20:30:40:50:60",
                DeviceKind::Ble,
                false,
                -39,
                "advertisement=0x020106 source=mock",
            ));
        }
        ScanPhase::Verification => {
            devices.push(raw(
                "mock-keyboard",
                phase,
                "Desk Keyboard",
                "20:20:30:40:50:60",
                DeviceKind::Classic,
                true,
                -51,
                "paired=true source=mock",
            ));
            devices.push(raw(
                "mock-door-sensor",
                phase,
                "Door Sensor",
                "40:20:30:40:50:60",
                DeviceKind::Ble,
                false,
                -74,
                "late-change=true source=mock",
            ));
        }
    }

    devices
}

fn raw(
    device_id: &str,
    phase: ScanPhase,
    name: &str,
    address: &str,
    kind: DeviceKind,
    is_paired: bool,
    rssi: i32,
    properties_summary: &str,
) -> RawObservation {
    RawObservation {
        device_id: device_id.to_string(),
        phase,
        name: name.to_string(),
        address: address.to_string(),
        kind,
        is_paired,
        rssi: Some(rssi),
        properties_summary: properties_summary.to_string(),
        observed_at: Local::now(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn mock_backend_reports_default_adapter() {
        let backend = MockBluetoothBackend::new();
        let adapter = backend.default_adapter().unwrap();
        assert_eq!(adapter.name, "Mock Bluetooth Adapter");
        assert_eq!(adapter.address, "02:00:5E:00:53:01");
    }

    #[test]
    fn mock_backend_emits_target_phase_observations() {
        let (tx, rx) = mpsc::channel();
        let mut backend = MockBluetoothBackend::new();

        backend.start(tx, ScanPhase::Target).unwrap();
        let observations = (0..3)
            .map(|_| rx.recv_timeout(Duration::from_secs(2)).unwrap())
            .collect::<Vec<_>>();
        backend.stop();

        assert!(observations
            .iter()
            .any(|obs| obs.device_id == "mock-target-speaker" && obs.phase == ScanPhase::Target));
    }
}
