use std::sync::mpsc::Sender;

use findbt_core::{HostAdapterInfo, RawObservation, ScanPhase};
use thiserror::Error;

#[cfg(any(
    feature = "mock-backend",
    not(any(target_os = "windows", target_os = "macos"))
))]
mod mock;

#[cfg(any(
    feature = "mock-backend",
    not(any(target_os = "windows", target_os = "macos"))
))]
pub use mock::MockBluetoothBackend as DefaultBluetoothBackend;
#[cfg(any(
    feature = "mock-backend",
    not(any(target_os = "windows", target_os = "macos"))
))]
pub use mock::MockBluetoothBackend;

pub trait BluetoothBackend {
    fn default_adapter(&self) -> Option<HostAdapterInfo>;
    fn start(&mut self, tx: Sender<RawObservation>, phase: ScanPhase) -> Result<(), BackendError>;
    fn stop(&mut self);
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("a scan is already running")]
    AlreadyRunning,
    #[error("Bluetooth backend is not available on this platform/build")]
    UnsupportedPlatform,
    #[error("Bluetooth backend failed: {0}")]
    Other(String),
}

#[cfg(all(target_os = "windows", not(feature = "mock-backend")))]
mod windows;

#[cfg(all(target_os = "windows", not(feature = "mock-backend")))]
pub use windows::WindowsBluetoothBackend as DefaultBluetoothBackend;

#[cfg(all(target_os = "macos", not(feature = "mock-backend")))]
mod macos;

#[cfg(all(target_os = "macos", not(feature = "mock-backend")))]
pub use macos::MacosBluetoothBackend as DefaultBluetoothBackend;
