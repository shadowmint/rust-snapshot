mod helpers;

use self::error::CaptureError;
use rascam::*;
use std::mem::size_of;
use std::os::raw::{c_char, c_int, c_uchar};
use std::process::exit;
use std::ptr::{null, null_mut};

pub struct CaptureSettings {
    resolution: (u32, u32),
}

pub struct Capture {
    pub settings: CaptureSettings,
    camera: Option<SeriousCamera>,
}

impl Capture {
    pub fn new(settings: CaptureSettings) -> Capture {
        Capture {
            settings,
            camera: None,
        }
    }

    pub fn init(&mut self) -> Result<(), CaptureError> {
        let info = info()?;
        if info.cameras.len() < 1 {
            return Err(CaptureError::NoCamerasFound);
        }

        let mut camera = SeriousCamera::new()?;
        camera.set_camera_num(0)?;
        camera.create_encoder()?;
        camera.enable_control_port(true)?;
        camera.set_camera_params(&info.cameras[0])?;

        let settings = CameraSettings {
            encoding: MMAL_ENCODING_RGB24,
            width: self.settings.resolution.0,
            height: self.settings.resolution.1,
            iso: ISO_AUTO,
            zero_copy: true,
            use_encoder: false,
        };

        camera.set_camera_format(&settings)?;
        camera.enable()?;
        camera.create_pool()?;
        camera.create_preview()?;
        camera.connect_preview()?;
        camera.enable_preview()?;

        self.camera = Some(camera);
        Ok(())
    }

    pub fn shutdown(self) {}

    pub fn get_buffer_size(&self) -> Result<usize, CaptureError> {
        Ok((self.settings.resolution.0 * self.settings.resolution.1 * 3) as usize)
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<(), CaptureError> {
        if let Some(camera) = self.camera.as_mut() {
            let receiver = <SeriousCamera>::take(camera)?;
            if let Some(camera_buffer) = receiver.recv()? {
                buffer.copy_from_slice(camera_buffer.get_bytes());
                Ok(())
            } else {
                Err(CaptureError::NoBuffer)
            }
        } else {
            Err(CaptureError::NotReady)
        }
    }
}

mod error {
    use rascam::CameraError;
    use std::sync::mpsc;

    #[derive(Debug)]
    pub enum CaptureError {
        NotReady,
        NoBuffer,
        NoCamerasFound,
        CameraError(String),
    }

    impl From<CameraError> for CaptureError {
        fn from(err: CameraError) -> Self {
            CaptureError::CameraError(format!("{}", err))
        }
    }

    impl From<mpsc::RecvError> for CaptureError {
        fn from(err: mpsc::RecvError) -> Self {
            CaptureError::CameraError(format!("{}", err))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::helpers::as_rgb_image;
    use crate::{Capture, CaptureSettings};
    use std::thread::sleep;
    use std::time::Duration;

    #[cfg(target_os = "linux")]
    #[test]
    fn capture_single_frame() {
        let size = (1280, 720);
        let mut capture = Capture::new(CaptureSettings { resolution: size });

        let buffer_size = capture.get_buffer_size().unwrap();
        let mut buffer = vec![0u8; buffer_size];

        capture.init().unwrap();
        capture.read(buffer.as_mut()).unwrap();
        capture.shutdown();

        let rgb_image = as_rgb_image(buffer.as_slice(), size.0, size.1).unwrap();
        rgb_image.save("snapshot.png").unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn capture_several_frames() {
        let size = (1280, 720);
        let mut capture = Capture::new(CaptureSettings { resolution: size });

        let buffer_size = capture.get_buffer_size().unwrap();
        let mut buffer = vec![0u8; buffer_size];

        capture.init().unwrap();

        for i in 0..10 {
            capture.read(buffer.as_mut()).unwrap();
            println!("Captured frame {}", i);

            let rgb_image = as_rgb_image(buffer.as_slice(), size.0, size.1).unwrap();
            rgb_image.save(format!("snapshot_{}.png", i)).unwrap();
            println!("Saved frame {}", i);

            sleep(Duration::from_millis(100))
        }

        capture.shutdown();
    }
}
