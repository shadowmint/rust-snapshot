use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
pub struct Manifest {
    pub config: ManifestConfig,

    pub export: ManifestExport,

    /// Device settings
    pub settings: HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ManifestExport {
    /// The path to export
    pub export_file: String,

    /// The framerate to export with
    pub export_framerate: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct ManifestConfig {
    pub output_folder: String,
    pub log_folder: String,

    /// What lock file should be used to control the app?
    pub lock_file: String,

    /// How long between frames in ms.
    pub sample_interval: u64,

    /// How long to sleep before checking for a new frame in ms.
    pub sample_idle: u64,

    /// Should the application use NTP to get a 'real' time before starting.
    pub use_ntp: bool,
}
