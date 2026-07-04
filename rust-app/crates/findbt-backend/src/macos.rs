use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc, Mutex,
};

use chrono::Local;
use findbt_core::{normalize_address, DeviceKind, HostAdapterInfo, RawObservation, ScanPhase};
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, ProtocolObject},
    AnyThread, DefinedClass,
};
use objc2_core_bluetooth::{
    CBAdvertisementDataIsConnectable, CBAdvertisementDataLocalNameKey, CBCentralManager,
    CBCentralManagerDelegate, CBManagerState, CBPeripheral,
};
use objc2_foundation::{NSDictionary, NSNumber, NSObjectProtocol, NSString};
use objc2_io_bluetooth::{
    IOBluetoothDevice, IOBluetoothDeviceInquiry, IOBluetoothDeviceInquiryDelegate,
    IOBluetoothDeviceSearchTypesBits, IOBluetoothHostController,
};
use serde_json::Value;

use crate::{BackendError, BluetoothBackend};

pub struct MacosBluetoothBackend {
    inner: Option<MacosScanInner>,
}

struct MacosScanInner {
    delegate: Retained<MacBluetoothDelegate>,
    central: Retained<CBCentralManager>,
    inquiry: Retained<IOBluetoothDeviceInquiry>,
}

#[derive(Clone)]
struct DelegateState {
    tx: Sender<RawObservation>,
    phase: ScanPhase,
    running: Arc<AtomicBool>,
}

#[derive(Clone)]
struct MacBluetoothDelegateIvars {
    state: Arc<Mutex<DelegateState>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = MacBluetoothDelegateIvars]
    struct MacBluetoothDelegate;

    unsafe impl NSObjectProtocol for MacBluetoothDelegate {}

    unsafe impl CBCentralManagerDelegate for MacBluetoothDelegate {
        #[unsafe(method(centralManagerDidUpdateState:))]
        fn central_manager_did_update_state(&self, central: &CBCentralManager) {
            let state = self.ivars().state.lock().ok().map(|state| state.clone());
            let Some(state) = state else {
                return;
            };
            if !state.running.load(Ordering::SeqCst) {
                return;
            }

            let manager_state = unsafe { central.state() };
            if manager_state == CBManagerState::PoweredOn {
                unsafe {
                    central.scanForPeripheralsWithServices_options(None, None);
                }
            } else {
                let _ = state.tx.send(RawObservation {
                    device_id: "macos-corebluetooth-state".to_string(),
                    phase: state.phase,
                    name: "CoreBluetooth central is not powered on".to_string(),
                    address: String::new(),
                    kind: DeviceKind::Ble,
                    is_paired: false,
                    rssi: None,
                    properties_summary: format!("state={manager_state:?}"),
                    observed_at: Local::now(),
                });
            }
        }

        #[unsafe(method(centralManager:didDiscoverPeripheral:advertisementData:RSSI:))]
        fn central_manager_did_discover_peripheral(
            &self,
            _central: &CBCentralManager,
            peripheral: &CBPeripheral,
            advertisement_data: &NSDictionary<NSString, AnyObject>,
            rssi: &NSNumber,
        ) {
            let state = self.ivars().state.lock().ok().map(|state| state.clone());
            let Some(state) = state else {
                return;
            };
            if !state.running.load(Ordering::SeqCst) {
                return;
            }

            let observation = ble_observation(peripheral, advertisement_data, rssi, state.phase);
            let _ = state.tx.send(observation);
        }
    }

    unsafe impl IOBluetoothDeviceInquiryDelegate for MacBluetoothDelegate {
        #[unsafe(method(deviceInquiryDeviceFound:device:))]
        fn device_inquiry_device_found(
            &self,
            _sender: Option<&IOBluetoothDeviceInquiry>,
            device: Option<&IOBluetoothDevice>,
        ) {
            let state = self.ivars().state.lock().ok().map(|state| state.clone());
            let Some((state, device)) = state.zip(device) else {
                return;
            };
            if !state.running.load(Ordering::SeqCst) {
                return;
            }

            let _ = state.tx.send(classic_observation(device, state.phase));
        }

        #[unsafe(method(deviceInquiryDeviceNameUpdated:device:devicesRemaining:))]
        fn device_inquiry_device_name_updated(
            &self,
            _sender: Option<&IOBluetoothDeviceInquiry>,
            device: Option<&IOBluetoothDevice>,
            _devices_remaining: u32,
        ) {
            let state = self.ivars().state.lock().ok().map(|state| state.clone());
            let Some((state, device)) = state.zip(device) else {
                return;
            };
            if !state.running.load(Ordering::SeqCst) {
                return;
            }

            let _ = state.tx.send(classic_observation(device, state.phase));
        }

        #[unsafe(method(deviceInquiryComplete:error:aborted:))]
        fn device_inquiry_complete(
            &self,
            sender: Option<&IOBluetoothDeviceInquiry>,
            error: i32,
            aborted: bool,
        ) {
            let state = self.ivars().state.lock().ok().map(|state| state.clone());
            let Some(state) = state else {
                return;
            };
            if !state.running.load(Ordering::SeqCst) {
                return;
            }

            if error == 0 && !aborted {
                if let Some(sender) = sender {
                    unsafe {
                        let _ = sender.start();
                    }
                }
            } else if error != 0 && !aborted {
                let _ = state.tx.send(RawObservation {
                    device_id: "macos-iobluetooth-inquiry-error".to_string(),
                    phase: state.phase,
                    name: "IOBluetooth inquiry stopped with an error".to_string(),
                    address: String::new(),
                    kind: DeviceKind::Classic,
                    is_paired: false,
                    rssi: None,
                    properties_summary: format!("IOReturn={error}"),
                    observed_at: Local::now(),
                });
            }
        }
    }
);

impl MacBluetoothDelegate {
    fn new(
        tx: Sender<RawObservation>,
        phase: ScanPhase,
        running: Arc<AtomicBool>,
    ) -> Retained<Self> {
        let state = DelegateState { tx, phase, running };
        let this = Self::alloc().set_ivars(MacBluetoothDelegateIvars {
            state: Arc::new(Mutex::new(state)),
        });
        unsafe { msg_send![super(this), init] }
    }
}

impl MacosBluetoothBackend {
    pub fn new() -> Self {
        Self { inner: None }
    }
}

impl Default for MacosBluetoothBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothBackend for MacosBluetoothBackend {
    fn default_adapter(&self) -> Option<HostAdapterInfo> {
        host_controller_info().or_else(system_profiler_host_info)
    }

    fn start(&mut self, tx: Sender<RawObservation>, phase: ScanPhase) -> Result<(), BackendError> {
        if self.inner.is_some() {
            return Err(BackendError::AlreadyRunning);
        }

        let running = Arc::new(AtomicBool::new(true));
        let delegate = MacBluetoothDelegate::new(tx.clone(), phase, Arc::clone(&running));
        emit_paired_classic_devices(&tx, phase);

        let central = unsafe {
            CBCentralManager::initWithDelegate_queue(
                CBCentralManager::alloc(),
                Some(ProtocolObject::from_ref(&*delegate)),
                None,
            )
        };
        if unsafe { central.state() } == CBManagerState::PoweredOn {
            unsafe {
                central.scanForPeripheralsWithServices_options(None, None);
            }
        }

        let inquiry = unsafe {
            IOBluetoothDeviceInquiry::inquiryWithDelegate(Some((*delegate).as_ref())).ok_or_else(
                || BackendError::Other("IOBluetooth inquiry could not be created".to_string()),
            )?
        };
        unsafe {
            inquiry.setUpdateNewDeviceNames(true);
            inquiry.setInquiryLength(10);
            inquiry.setSearchType(IOBluetoothDeviceSearchTypesBits::Classic.0);
            let result = inquiry.start();
            if result != 0 {
                return Err(BackendError::Other(format!(
                    "IOBluetooth inquiry could not start: IOReturn={result}"
                )));
            }
        }

        self.inner = Some(MacosScanInner {
            delegate,
            central,
            inquiry,
        });
        Ok(())
    }

    fn stop(&mut self) {
        let Some(inner) = self.inner.take() else {
            return;
        };

        if let Ok(state) = inner.delegate.ivars().state.lock() {
            state.running.store(false, Ordering::SeqCst);
        }

        unsafe {
            inner.central.stopScan();
            let _ = inner.inquiry.stop();
            inner.inquiry.setDelegate(None);
            inner.central.setDelegate(None);
        }
    }
}

impl Drop for MacosBluetoothBackend {
    fn drop(&mut self) {
        self.stop();
    }
}

fn host_controller_info() -> Option<HostAdapterInfo> {
    let controller = unsafe { IOBluetoothHostController::defaultController()? };
    let name = unsafe { controller.nameAsString() }
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "macOS Bluetooth Adapter".to_string());
    let address = unsafe { controller.addressAsString() }
        .map(|value| normalize_address(&value.to_string()))
        .unwrap_or_default();
    Some(HostAdapterInfo {
        name,
        address,
        ..Default::default()
    })
}

fn emit_paired_classic_devices(tx: &Sender<RawObservation>, phase: ScanPhase) {
    let Some(devices) = (unsafe { IOBluetoothDevice::pairedDevices() }) else {
        return;
    };

    for device in devices.iter() {
        let Some(device) = device.downcast_ref::<IOBluetoothDevice>() else {
            continue;
        };
        let _ = tx.send(classic_observation(device, phase));
    }
}

fn ble_observation(
    peripheral: &CBPeripheral,
    advertisement_data: &NSDictionary<NSString, AnyObject>,
    rssi: &NSNumber,
    phase: ScanPhase,
) -> RawObservation {
    let identifier = unsafe { peripheral.identifier() }.UUIDString().to_string();
    let local_name = advertisement_string(advertisement_data, unsafe {
        CBAdvertisementDataLocalNameKey
    });
    let name = local_name
        .or_else(|| unsafe { peripheral.name() }.map(|value| value.to_string()))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "(advertising BLE peripheral)".to_string());
    let rssi = rssi.intValue();
    let rssi = if rssi == 127 { None } else { Some(rssi) };
    let connectable = advertisement_bool(advertisement_data, unsafe {
        CBAdvertisementDataIsConnectable
    });

    RawObservation {
        device_id: format!("corebluetooth:{identifier}"),
        phase,
        name,
        address: String::new(),
        kind: DeviceKind::Ble,
        is_paired: false,
        rssi,
        properties_summary: format!(
            "CoreBluetooth identifier={identifier}; connectable={}; advertisementKeys={}",
            connectable
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            advertisement_data.len()
        ),
        observed_at: Local::now(),
    }
}

fn classic_observation(device: &IOBluetoothDevice, phase: ScanPhase) -> RawObservation {
    let address = unsafe { device.addressString() }
        .map(|value| normalize_address(&value.to_string()))
        .unwrap_or_default();
    let name = unsafe { device.nameOrAddress() }
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "(unnamed classic Bluetooth device)".to_string());
    let is_paired = unsafe { device.isPaired() };
    let is_connected = unsafe { device.isConnected() };
    let rssi = if is_connected {
        Some(unsafe { device.rawRSSI() } as i32)
    } else {
        None
    };

    RawObservation {
        device_id: if address.is_empty() {
            format!("iobluetooth:{name}")
        } else {
            format!("iobluetooth:{address}")
        },
        phase,
        name,
        address,
        kind: DeviceKind::Classic,
        is_paired,
        rssi,
        properties_summary: format!(
            "IOBluetooth classic; connected={is_connected}; paired={is_paired}"
        ),
        observed_at: Local::now(),
    }
}

fn advertisement_string(
    advertisement_data: &NSDictionary<NSString, AnyObject>,
    key: &NSString,
) -> Option<String> {
    advertisement_data
        .objectForKey(key)
        .and_then(|value| value.downcast::<NSString>().ok())
        .map(|value| value.to_string())
}

fn advertisement_bool(
    advertisement_data: &NSDictionary<NSString, AnyObject>,
    key: &NSString,
) -> Option<bool> {
    advertisement_data
        .objectForKey(key)
        .and_then(|value| value.downcast::<NSNumber>().ok())
        .map(|value| value.boolValue())
}

fn system_profiler_host_info() -> Option<HostAdapterInfo> {
    let output = std::process::Command::new("/usr/sbin/system_profiler")
        .args(["SPBluetoothDataType", "-json"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value: Value = serde_json::from_slice(&output.stdout).ok()?;
    let controller = value
        .pointer("/SPBluetoothDataType/0/controller_properties")
        .or_else(|| find_object_with_key(&value, "controller_address"))?;

    let name = string_for_any_key(controller, &["controller_name", "name", "_name"])
        .unwrap_or_else(|| "macOS Bluetooth Adapter".to_string());
    let address = string_for_any_key(controller, &["controller_address", "address"])
        .map(|value| normalize_address(&value))
        .unwrap_or_default();

    Some(HostAdapterInfo {
        name,
        address,
        ..Default::default()
    })
}

fn find_object_with_key<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            if map.contains_key(key) {
                Some(value)
            } else {
                map.values()
                    .find_map(|child| find_object_with_key(child, key))
            }
        }
        Value::Array(items) => items
            .iter()
            .find_map(|child| find_object_with_key(child, key)),
        _ => None,
    }
}

fn string_for_any_key(value: &Value, keys: &[&str]) -> Option<String> {
    let object = value.as_object()?;
    keys.iter().find_map(|key| {
        object
            .get(*key)
            .and_then(Value::as_str)
            .map(ToString::to_string)
    })
}
