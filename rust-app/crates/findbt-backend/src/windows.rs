use std::sync::{mpsc::Sender, Arc, Mutex};

use chrono::Local;
use findbt_core::{DeviceKind, HostAdapterInfo, RawObservation, ScanPhase};
use windows::{
    Devices::{
        Bluetooth::{
            Advertisement::{
                BluetoothLEAdvertisementReceivedEventArgs, BluetoothLEAdvertisementWatcher,
                BluetoothLEAdvertisementWatcherStatus, BluetoothLEScanningMode,
            },
            BluetoothAdapter, BluetoothDevice, BluetoothLEDevice,
        },
        Enumeration::{
            DeviceInformation, DeviceInformationKind, DeviceInformationUpdate, DeviceWatcher,
            DeviceWatcherStatus,
        },
    },
    Foundation::TypedEventHandler,
};
use windows_core::{IInspectable, Interface, HSTRING};

use crate::{BackendError, BluetoothBackend};

const REQUESTED_PROPERTIES: [&str; 4] = [
    "System.Devices.Aep.DeviceAddress",
    "System.Devices.Aep.IsPaired",
    "System.Devices.Aep.Bluetooth.Le.IsConnectable",
    "System.Devices.Aep.SignalStrength",
];

pub struct WindowsBluetoothBackend {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    watchers: Vec<WatcherRegistration>,
    advertisement: Option<AdvertisementRegistration>,
}

struct WatcherRegistration {
    watcher: DeviceWatcher,
    added: i64,
    updated: i64,
    removed: i64,
    completed: i64,
    stopped: i64,
}

struct AdvertisementRegistration {
    watcher: BluetoothLEAdvertisementWatcher,
    received: i64,
    stopped: i64,
}

impl WindowsBluetoothBackend {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
        }
    }
}

impl Default for WindowsBluetoothBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothBackend for WindowsBluetoothBackend {
    fn default_adapter(&self) -> Option<HostAdapterInfo> {
        let adapter = BluetoothAdapter::GetDefaultAsync().ok()?.join().ok()?;
        let address = adapter
            .BluetoothAddress()
            .map(format_bluetooth_address)
            .unwrap_or_default();
        let name = adapter
            .DeviceId()
            .map(|value| value.to_string())
            .unwrap_or_else(|_| "Windows Bluetooth Adapter".to_string());

        Some(HostAdapterInfo {
            name,
            address,
            ..Default::default()
        })
    }

    fn start(&mut self, tx: Sender<RawObservation>, phase: ScanPhase) -> Result<(), BackendError> {
        self.stop();

        let mut state = self
            .state
            .lock()
            .map_err(|_| BackendError::Other("Windows backend state lock poisoned".to_string()))?;

        add_watcher(
            &mut state,
            BluetoothLEDevice::GetDeviceSelectorFromPairingState(false)
                .map_err(to_backend_error)?,
            DeviceKind::Ble,
            "Bluetooth LE",
            tx.clone(),
            phase,
        )?;
        add_watcher(
            &mut state,
            BluetoothLEDevice::GetDeviceSelectorFromPairingState(true).map_err(to_backend_error)?,
            DeviceKind::Ble,
            "Bluetooth LE",
            tx.clone(),
            phase,
        )?;
        add_watcher(
            &mut state,
            BluetoothDevice::GetDeviceSelectorFromPairingState(false).map_err(to_backend_error)?,
            DeviceKind::Classic,
            "Bluetooth Classic",
            tx.clone(),
            phase,
        )?;
        add_watcher(
            &mut state,
            BluetoothDevice::GetDeviceSelectorFromPairingState(true).map_err(to_backend_error)?,
            DeviceKind::Classic,
            "Bluetooth Classic",
            tx.clone(),
            phase,
        )?;
        add_advertisement_watcher(&mut state, tx, phase)?;

        for registration in &state.watchers {
            registration.watcher.Start().map_err(to_backend_error)?;
        }
        if let Some(registration) = &state.advertisement {
            registration.watcher.Start().map_err(to_backend_error)?;
        }

        Ok(())
    }

    fn stop(&mut self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        for registration in state.watchers.drain(..) {
            let _ = registration.watcher.RemoveAdded(registration.added);
            let _ = registration.watcher.RemoveUpdated(registration.updated);
            let _ = registration.watcher.RemoveRemoved(registration.removed);
            let _ = registration
                .watcher
                .RemoveEnumerationCompleted(registration.completed);
            let _ = registration.watcher.RemoveStopped(registration.stopped);
            if matches!(
                registration.watcher.Status(),
                Ok(DeviceWatcherStatus::Started | DeviceWatcherStatus::EnumerationCompleted)
            ) {
                let _ = registration.watcher.Stop();
            }
        }

        if let Some(registration) = state.advertisement.take() {
            let _ = registration.watcher.RemoveReceived(registration.received);
            let _ = registration.watcher.RemoveStopped(registration.stopped);
            if matches!(
                registration.watcher.Status(),
                Ok(BluetoothLEAdvertisementWatcherStatus::Started)
            ) {
                let _ = registration.watcher.Stop();
            }
        }
    }
}

impl Drop for WindowsBluetoothBackend {
    fn drop(&mut self) {
        self.stop();
    }
}

fn add_watcher(
    state: &mut State,
    selector: HSTRING,
    kind: DeviceKind,
    kind_label: &'static str,
    tx: Sender<RawObservation>,
    phase: ScanPhase,
) -> Result<(), BackendError> {
    let requested_properties = windows_collections::IIterable::<HSTRING>::from(
        REQUESTED_PROPERTIES
            .iter()
            .map(|property| HSTRING::from(*property))
            .collect::<Vec<_>>(),
    );
    let watcher = DeviceInformation::CreateWatcherWithKindAqsFilterAndAdditionalProperties(
        &selector,
        &requested_properties,
        DeviceInformationKind::AssociationEndpoint,
    )
    .map_err(to_backend_error)?;

    let added_tx = tx.clone();
    let added = watcher
        .Added(&TypedEventHandler::<DeviceWatcher, DeviceInformation>::new(
            move |_sender, device| {
                if let Some(device) = device.as_ref() {
                    let _ = added_tx.send(observation_from_device(device, phase, kind, kind_label));
                }
                Ok(())
            },
        ))
        .map_err(to_backend_error)?;

    let updated_tx = tx.clone();
    let updated = watcher
        .Updated(
            &TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(
                move |_sender, update| {
                    if let Some(update) = update.as_ref() {
                        let _ = updated_tx
                            .send(observation_from_update(update, phase, kind, kind_label));
                    }
                    Ok(())
                },
            ),
        )
        .map_err(to_backend_error)?;

    let removed = watcher
        .Removed(
            &TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(
                move |_sender, _update| Ok(()),
            ),
        )
        .map_err(to_backend_error)?;
    let completed = watcher
        .EnumerationCompleted(&TypedEventHandler::<DeviceWatcher, IInspectable>::new(
            move |_sender, _args| Ok(()),
        ))
        .map_err(to_backend_error)?;
    let stopped = watcher
        .Stopped(&TypedEventHandler::<DeviceWatcher, IInspectable>::new(
            move |_sender, _args| Ok(()),
        ))
        .map_err(to_backend_error)?;

    state.watchers.push(WatcherRegistration {
        watcher,
        added,
        updated,
        removed,
        completed,
        stopped,
    });

    Ok(())
}

fn add_advertisement_watcher(
    state: &mut State,
    tx: Sender<RawObservation>,
    phase: ScanPhase,
) -> Result<(), BackendError> {
    let watcher = BluetoothLEAdvertisementWatcher::new().map_err(to_backend_error)?;
    watcher
        .SetScanningMode(BluetoothLEScanningMode::Active)
        .map_err(to_backend_error)?;

    let received = watcher
        .Received(&TypedEventHandler::<
            BluetoothLEAdvertisementWatcher,
            BluetoothLEAdvertisementReceivedEventArgs,
        >::new(move |_sender, args| {
            if let Some(args) = args.as_ref() {
                let _ = tx.send(observation_from_advertisement(args, phase));
            }
            Ok(())
        }))
        .map_err(to_backend_error)?;
    let stopped = watcher
        .Stopped(&TypedEventHandler::new(move |_sender, _args| Ok(())))
        .map_err(to_backend_error)?;

    state.advertisement = Some(AdvertisementRegistration {
        watcher,
        received,
        stopped,
    });

    Ok(())
}

fn observation_from_device(
    device: &DeviceInformation,
    phase: ScanPhase,
    kind: DeviceKind,
    kind_label: &str,
) -> RawObservation {
    let id = device
        .Id()
        .map(|value| value.to_string())
        .unwrap_or_else(|_| "unknown-device-id".to_string());
    let name = device
        .Name()
        .map(|value| value.to_string())
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "(unnamed)".to_string());
    let properties = device.Properties().ok();
    let address = properties
        .as_ref()
        .and_then(|properties| property_string(properties, "System.Devices.Aep.DeviceAddress"))
        .unwrap_or_default();
    let is_paired = properties
        .as_ref()
        .and_then(|properties| property_bool(properties, "System.Devices.Aep.IsPaired"))
        .unwrap_or(false);
    let rssi = properties
        .as_ref()
        .and_then(|properties| property_i32(properties, "System.Devices.Aep.SignalStrength"));
    let properties_summary = properties
        .as_ref()
        .map(summarize_properties)
        .unwrap_or_default();

    RawObservation {
        device_id: id,
        phase,
        name,
        address,
        kind,
        is_paired,
        rssi,
        properties_summary: if properties_summary.is_empty() {
            format!("Watcher={kind_label}")
        } else {
            format!("Watcher={kind_label}; {properties_summary}")
        },
        observed_at: Local::now(),
    }
}

fn observation_from_update(
    update: &DeviceInformationUpdate,
    phase: ScanPhase,
    kind: DeviceKind,
    kind_label: &str,
) -> RawObservation {
    let id = update
        .Id()
        .map(|value| value.to_string())
        .unwrap_or_else(|_| "unknown-device-id".to_string());
    let properties = update.Properties().ok();
    let address = properties
        .as_ref()
        .and_then(|properties| property_string(properties, "System.Devices.Aep.DeviceAddress"))
        .unwrap_or_default();
    let is_paired = properties
        .as_ref()
        .and_then(|properties| property_bool(properties, "System.Devices.Aep.IsPaired"))
        .unwrap_or(false);
    let rssi = properties
        .as_ref()
        .and_then(|properties| property_i32(properties, "System.Devices.Aep.SignalStrength"));
    let properties_summary = properties
        .as_ref()
        .map(summarize_properties)
        .unwrap_or_default();

    RawObservation {
        device_id: id,
        phase,
        name: "(updated device)".to_string(),
        address,
        kind,
        is_paired,
        rssi,
        properties_summary: if properties_summary.is_empty() {
            format!("Watcher={kind_label}; update=true")
        } else {
            format!("Watcher={kind_label}; update=true; {properties_summary}")
        },
        observed_at: Local::now(),
    }
}

fn observation_from_advertisement(
    args: &BluetoothLEAdvertisementReceivedEventArgs,
    phase: ScanPhase,
) -> RawObservation {
    let address = args
        .BluetoothAddress()
        .map(format_bluetooth_address)
        .unwrap_or_default();
    let name = args
        .Advertisement()
        .ok()
        .and_then(|advertisement| advertisement.LocalName().ok())
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "(advertising)".to_string());
    let rssi = args.RawSignalStrengthInDBm().ok().map(i32::from);
    let advertisement_type = args
        .AdvertisementType()
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|_| "unknown".to_string());

    RawObservation {
        device_id: format!("ble-advertisement:{address}"),
        phase,
        name,
        address,
        kind: DeviceKind::Ble,
        is_paired: false,
        rssi,
        properties_summary: format!("AdvertisementType={advertisement_type}"),
        observed_at: Local::now(),
    }
}

fn property_string(
    properties: &windows_collections::IMapView<HSTRING, IInspectable>,
    key: &str,
) -> Option<String> {
    let key = HSTRING::from(key);
    let value = properties.Lookup(&key).ok()?;
    value
        .cast::<windows::Foundation::IPropertyValue>()
        .ok()?
        .GetString()
        .ok()
        .map(|value| value.to_string())
}

fn property_bool(
    properties: &windows_collections::IMapView<HSTRING, IInspectable>,
    key: &str,
) -> Option<bool> {
    let key = HSTRING::from(key);
    let value = properties.Lookup(&key).ok()?;
    value
        .cast::<windows::Foundation::IPropertyValue>()
        .ok()?
        .GetBoolean()
        .ok()
}

fn property_i32(
    properties: &windows_collections::IMapView<HSTRING, IInspectable>,
    key: &str,
) -> Option<i32> {
    let key = HSTRING::from(key);
    let value = properties.Lookup(&key).ok()?;
    let property = value.cast::<windows::Foundation::IPropertyValue>().ok()?;
    property
        .GetInt32()
        .ok()
        .or_else(|| property.GetInt16().ok().map(i32::from))
}

fn summarize_properties(
    properties: &windows_collections::IMapView<HSTRING, IInspectable>,
) -> String {
    let mut summary = Vec::new();
    for key in [
        "System.Devices.Aep.DeviceAddress",
        "System.Devices.Aep.IsPaired",
        "System.Devices.Aep.Bluetooth.Le.IsConnectable",
        "System.Devices.Aep.SignalStrength",
    ] {
        if let Some(value) = property_string(properties, key) {
            summary.push(format!("{key}={value}"));
        } else if let Some(value) = property_bool(properties, key) {
            summary.push(format!("{key}={value}"));
        } else if let Some(value) = property_i32(properties, key) {
            summary.push(format!("{key}={value}"));
        }
    }
    summary.join("; ")
}

fn format_bluetooth_address(address: u64) -> String {
    let hex = format!("{address:012X}");
    (0..6)
        .map(|index| &hex[index * 2..index * 2 + 2])
        .collect::<Vec<_>>()
        .join(":")
}

fn to_backend_error(err: windows_core::Error) -> BackendError {
    BackendError::Other(err.to_string())
}
