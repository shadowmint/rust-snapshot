use crate::encoding::Encoding;
use crate::hardware::error::HardwareError;
use crate::hardware::{CameraLike, Frame};
use crate::resources::ConfigMap;
use rust_ffmpeg_capture::{Capture, CaptureSettings};
use toml::from_str;

pub struct AvCamera {
    buffer: Option<Vec<u8>>,
    capture: Option<Capture>,
    encoder: Encoding,
}

impl Default for AvCamera {
    fn default() -> Self {
        AvCamera {
            capture: None,
            buffer: None,
            encoder: Encoding::new(),
        }
    }
}

impl AvCamera {
    pub fn new() -> AvCamera {
        Default::default()
    }

    fn as_resolution_tuple(value: &str) -> Result<(u32, u32), HardwareError> {
        let parts: Vec<String> = value.split("x").map(|v| v.to_string()).collect();
        if parts.len() != 2 {
            return Err(HardwareError::InvalidSettings(format!(
                "{} is not a valid resolution; use the format AAAxBBB, eg. 640x480",
                value
            )));
        }

        let p1 = str::parse::<u32>(&parts[0]);
        let p1_u32 = match p1 {
            Ok(v) => v,
            Err(err) => {
                return Err(HardwareError::InvalidSettings(format!(
                    "{} in {} is not a valid resolution; use the format AAAxBBB, eg. 640x480",
                    &parts[0], value
                )));
            }
        };

        let p2 = str::parse::<u32>(&parts[1]);
        let p2_u32 = match p2 {
            Ok(v) => v,
            Err(_) => {
                return Err(HardwareError::InvalidSettings(format!(
                    "{} in {} is not a valid resolution; use the format AAAxBBB, eg. 640x480",
                    &parts[1], value
                )));
            }
        };

        Ok((p1_u32, p2_u32))
    }
}
impl CameraLike for AvCamera {
    fn initialize(&mut self, config: ConfigMap) -> Result<(), HardwareError> {
        let mut capture = Capture::new(CaptureSettings {
            backend: config.get_string("backend").unwrap_or("".to_string()),
            device: config.get_string("device").unwrap_or("".to_string()),
            resolution: AvCamera::as_resolution_tuple(
                &config
                    .get_string("resolution")
                    .unwrap_or_else(|| "640x480".to_string()),
            )?,
            framerate: config.get_u32("framerate").unwrap_or(1),
            pixel_format: config.get_string("pixel_format").unwrap_or("".to_string()),
        });

        let buffer_size = capture.get_buffer_size()?;
        let mut buffer = vec![0u8; buffer_size];
        self.buffer = Some(buffer);

        capture.init()?;
        self.capture = Some(capture);

        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), HardwareError> {
        if let Some(capture) = self.capture.take() {
            capture.shutdown();
        }
        self.capture = None;
        self.buffer = None;
        Ok(())
    }

    fn next(&mut self) -> Result<Frame, HardwareError> {
        if let Some(mut capture) = self.capture.as_mut() {
            if let Some(mut buffer) = self.buffer.as_mut() {
                capture.read(buffer.as_mut())?;
                let frame = self.encoder.frame_from_slice(
                    buffer.as_slice(),
                    capture.settings.resolution.0,
                    capture.settings.resolution.1,
                )?;
                return Ok(frame);
            }
        }
        Err(HardwareError::DeviceNoLongerAvailable(
            "Device state is invalid; call initialize() first".to_string(),
        ))
    }
}
