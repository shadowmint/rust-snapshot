use crate::app::error::AppError;
use crate::hardware::Frame;
use crate::resources::{ResourceFolder, TimeSnapshot};
use slog::error;
use slog::Logger;
use std::thread;

pub struct ImageLogger {
    output_folder: ResourceFolder,
    logger: Logger,
}

impl ImageLogger {
    pub fn new(output_folder: ResourceFolder, logger: Logger) -> ImageLogger {
        ImageLogger {
            output_folder,
            logger,
        }
    }

    pub(crate) fn save(&self, frame: Frame, timestamp: TimeSnapshot) -> Result<(), AppError> {
        let filename = format!("{}-{}.png", timestamp.timestamp, timestamp.utc.to_rfc2822());
        let filepath = self.output_folder.path(&filename)?;
        frame.save(filepath)?;
        Ok(())
    }
}
