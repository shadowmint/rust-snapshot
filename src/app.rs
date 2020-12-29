pub mod config;
mod image_logger;

use self::config::Manifest;
use self::error::AppError;
use crate::hardware::CameraFactory;
use crate::resources::{ConfigMap, LockFile, ResourceFolder, TimeProbe, TimeProbeConfig};
use slog::o;
use slog::{info, Drain, Duplicate, Logger};
use sloggers::file::FileLoggerBuilder;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;

use crate::app::image_logger::ImageLogger;
use std::time::Instant;

pub struct App {
    manifest: Manifest,
    output: ResourceFolder,
    logger: Logger,
    camera_config: ConfigMap,
}

impl App {
    pub fn new(manifest: Manifest) -> Result<App, AppError> {
        let output_folder = ResourceFolder::new(&manifest.config.output_folder).require()?;
        let log_folder = ResourceFolder::new(&manifest.config.log_folder).require()?;
        let config = App::create_camera_config(&manifest);
        Ok(App {
            manifest,
            output: output_folder,
            logger: App::create_logger(log_folder)?,
            camera_config: config,
        })
    }

    fn create_camera_config(manifest: &Manifest) -> ConfigMap {
        let mut config = ConfigMap::new();
        config.import(&manifest.settings);
        config
    }

    fn create_logger(log_folder: ResourceFolder) -> Result<Logger, AppError> {
        let mut builder = TerminalLoggerBuilder::new();
        builder.level(Severity::Debug);
        builder.destination(Destination::Stderr);
        let terminal_logger = builder.build()?;

        let mut builder = FileLoggerBuilder::new(log_folder.path("app.log")?);
        builder.level(Severity::Debug);
        builder.rotate_size(1024 * 1024 * 10);
        let file_logger = builder.build()?;

        let logger = Logger::root(Duplicate::new(file_logger, terminal_logger).fuse(), o!());
        Ok(logger)
    }

    pub fn run(&mut self) -> Result<(), AppError> {
        // Setup a camera based on the manifest
        let camera_factory = CameraFactory::new(self.camera_config.clone());
        let mut camera = camera_factory.create_camera()?;

        // Setup a probe based on the manifest
        let mut probe = TimeProbe::new(TimeProbeConfig {
            time_scale: 1f32,
            interval: self.manifest.config.sample_interval,
            idle: self.manifest.config.sample_idle,
            samples: -1,
        });

        if self.manifest.config.use_ntp {
            probe.sync_network_time("pool.ntp.org")?;
            info!(
                self.logger,
                "synchronized time to: UTC {}",
                probe.reference_time().to_rfc2822()
            )
        }

        // Setup an output handler from the manifest
        let image_logger = ImageLogger::new(self.output.clone(), self.logger.clone());

        // Keep running as long as the log lasts
        let run_lock = LockFile::new(&self.manifest.config.lock_file);
        run_lock.lock()?;

        for sample in probe {
            info!(self.logger, "snapshot start: {}", sample.utc.to_rfc2822());
            let sample_start = Instant::now();

            // Take a picture
            let frame = camera.next()?;
            info!(
                self.logger,
                "captured: {}x{} image",
                frame.width(),
                frame.height()
            );

            // Save the picture
            image_logger.save(frame, sample)?;

            let sample_end = Instant::now();
            let elapsed = (sample_end - sample_start).as_millis();
            info!(self.logger, "snapshot end: {}ms elapsed", elapsed);

            // TODO: Move this into probe so we poll at interval, not capture interval
            // Check for early exit
            if !run_lock.is_locked() {
                info!(self.logger, "lock removed, halting capture");
                break;
            }
        }

        camera.shutdown()?;
        Ok(())
    }
}

pub mod error {
    use crate::hardware::HardwareError;
    use crate::resources::{LockError, ResourceError, TimeProbe, TimeProbeError};
    use image::ImageError;
    use sloggers::Error;
    use std::fmt;

    #[derive(Debug)]
    pub enum AppError {
        LogInitFailed(String),
        InvalidResource(String),
        OutputError(String),
        DeviceFailed(String),
        LockFailed(String),
        NetworkFailed(String),
    }

    impl std::error::Error for AppError {}

    impl fmt::Display for AppError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl From<sloggers::Error> for AppError {
        fn from(err: Error) -> Self {
            AppError::LogInitFailed(format!("{:?}", err))
        }
    }

    impl From<ResourceError> for AppError {
        fn from(err: ResourceError) -> Self {
            AppError::InvalidResource(format!("{:?}", err))
        }
    }

    impl From<HardwareError> for AppError {
        fn from(err: HardwareError) -> Self {
            AppError::DeviceFailed(format!("{:?}", err))
        }
    }

    impl From<LockError> for AppError {
        fn from(err: LockError) -> Self {
            AppError::LockFailed(format!("{:?}", err))
        }
    }

    impl From<TimeProbeError> for AppError {
        fn from(err: TimeProbeError) -> Self {
            AppError::NetworkFailed(format!("{:?}", err))
        }
    }

    impl From<ImageError> for AppError {
        fn from(err: ImageError) -> Self {
            AppError::OutputError(format!("failed to save frame: {:?}", err))
        }
    }
}
