mod ffmpeg_camera;
mod mock_camera;

pub use self::error::HardwareError;
use crate::hardware::ffmpeg_camera::AvCamera;
use crate::hardware::mock_camera::MockCamera;
use crate::resources::ConfigMap;
use image::{ImageBuffer, Rgb};

pub type Frame<'a> = ImageBuffer<Rgb<u8>, &'a [u8]>;

pub trait CameraLike {
    /// Initialize the device and start streaming
    fn initialize(&mut self, config: ConfigMap) -> Result<(), HardwareError>;

    /// Stop streaming frames and shutdown
    fn shutdown(&mut self) -> Result<(), HardwareError>;

    /// Return the next image
    fn next(&mut self) -> Result<Frame, HardwareError>;
}

pub struct CameraFactory {
    config: ConfigMap,
}

impl CameraFactory {
    pub fn new(config: ConfigMap) -> CameraFactory {
        CameraFactory { config }
    }

    pub fn create_camera(&self) -> Result<Box<dyn CameraLike + 'static>, HardwareError> {
        let mut camera = if self.use_mock() {
            Box::new(MockCamera::new()) as Box<dyn CameraLike + 'static>
        } else {
            Box::new(AvCamera::new()) as Box<dyn CameraLike + 'static>
        };
        camera.initialize(self.config.clone())?;
        Ok(camera)
    }

    fn use_mock(&self) -> bool {
        self.config.flag("use_mock")
    }
}

mod error {
    use crate::encoding;
    use crate::resources::ResourceError;
    use image::ImageError;
    use std::fmt;
    use std::io;

    #[derive(Debug)]
    pub enum HardwareError {
        NotImplemented,
        DeviceNoLongerAvailable(String),
        FailedToEncodeFrame(String),
        InvalidSettings(String),
        DeviceFailed(String),
        IoError(String),
    }

    impl fmt::Display for HardwareError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl From<ResourceError> for HardwareError {
        fn from(err: ResourceError) -> Self {
            HardwareError::InvalidSettings(format!("{}", err))
        }
    }

    impl From<encoding::error::EncodingError> for HardwareError {
        fn from(err: encoding::error::EncodingError) -> Self {
            HardwareError::FailedToEncodeFrame(format!("{}", err))
        }
    }

    impl From<ImageError> for HardwareError {
        fn from(err: ImageError) -> Self {
            HardwareError::FailedToEncodeFrame(format!("{}", err))
        }
    }

    impl From<io::Error> for HardwareError {
        fn from(err: io::Error) -> Self {
            HardwareError::IoError(format!("{}", err))
        }
    }

    impl From<rust_ffmpeg_capture::CaptureError> for HardwareError {
        fn from(err: rust_ffmpeg_capture::CaptureError) -> Self {
            HardwareError::DeviceFailed(format!("{}", err))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CameraFactory;
    use crate::resources::ConfigMap;

    #[test]
    pub fn test_mock_factory() {
        let mut config = ConfigMap::new();
        config.set("use_mock", "1");
        config.set("use_mock_folder", "test/data/frames");

        let mut camera = CameraFactory::new(config).create_camera().unwrap();
        let frame = camera.next().unwrap();

        assert_eq!(frame.width(), 256);
        assert_eq!(frame.height(), 256);

        camera.shutdown().unwrap();
    }
}
