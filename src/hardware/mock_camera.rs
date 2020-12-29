use crate::encoding::Encoding;
use crate::hardware::error::HardwareError;
use crate::hardware::{CameraLike, Frame};
use crate::resources::{ConfigMap, ResourceFolder};
use image::io::Reader as ImageReader;
use std::fs::DirEntry;
use std::path::PathBuf;

pub struct MockCamera {
    offset: isize,
    repeat: bool,
    frames: Vec<DirEntry>,
    active: Option<Vec<u8>>,
}

impl Default for MockCamera {
    fn default() -> Self {
        MockCamera {
            repeat: false,
            offset: -1,
            frames: Vec::new(),
            active: None,
        }
    }
}

impl MockCamera {
    pub fn new() -> MockCamera {
        Default::default()
    }

    fn read_frame(&mut self, entry: PathBuf) -> Result<Frame, HardwareError> {
        let img = ImageReader::open(entry)?.decode()?.to_rgb8();
        let width = img.width();
        let height = img.height();
        let buffer = img.as_raw().clone();
        let encoding = Encoding::new();
        self.active = Some(buffer);
        if let Some(buffer_ref) = self.active.as_ref() {
            let buffer_slice = buffer_ref.as_ref();
            Ok(encoding.frame_from_slice(buffer_slice, width, height)?)
        } else {
            Err(HardwareError::DeviceNoLongerAvailable(
                "Invalid mock frame".to_string(),
            ))
        }
    }
}

impl CameraLike for MockCamera {
    fn initialize(&mut self, config: ConfigMap) -> Result<(), HardwareError> {
        if let Some(path) = config.get_string("use_mock_folder") {
            let resources = ResourceFolder::new(&path).require_existing()?;
            self.frames = resources.enumerate_files()?;
            self.repeat = config.flag("use_mock_repeat_frames");
            self.offset = -1;
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), HardwareError> {
        Ok(())
    }

    fn next(&mut self) -> Result<Frame, HardwareError> {
        self.offset += 1;
        if self.offset >= (self.frames.len() as isize) {
            if self.repeat {
                self.offset = 0;
            } else {
                return Err(HardwareError::DeviceNoLongerAvailable(
                    "Ran out of mock frames".to_string(),
                ));
            }
        }
        let entry_path = self.frames[(self.offset as usize)].path();
        Ok(self.read_frame(entry_path)?)
    }
}
