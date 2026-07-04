use crate::{
    models::{CaseMetadata, HostAdapterInfo, ScanPhase, ScanPhaseRun},
    registry::{DeviceRegistry, RawObservation},
};

#[derive(Debug, Clone)]
pub struct CaptureSession {
    pub metadata: CaseMetadata,
    pub host: HostAdapterInfo,
    pub phase_runs: Vec<ScanPhaseRun>,
    pub registry: DeviceRegistry,
    pub raw_log: Vec<RawObservation>,
}

impl CaptureSession {
    pub fn new(metadata: CaseMetadata, host: HostAdapterInfo) -> Self {
        Self {
            metadata,
            host,
            phase_runs: Vec::new(),
            registry: DeviceRegistry::new(),
            raw_log: Vec::new(),
        }
    }

    pub fn record(&mut self, observation: RawObservation) {
        self.registry.record(&observation);
        self.raw_log.push(observation);
    }

    pub fn reset_capture(&mut self) {
        self.phase_runs.clear();
        self.registry.clear();
        self.raw_log.clear();
    }

    pub fn phase_run_for(&self, phase: ScanPhase) -> Option<&ScanPhaseRun> {
        self.phase_runs.iter().rev().find(|run| run.phase == phase)
    }
}
