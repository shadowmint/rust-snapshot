mod ffmpeg_exporter;

use crate::encoding::error::EncodingError;
use crate::encoding::ffmpeg_exporter::invoke_ffmpeg_cli;
use crate::hardware::Frame;
use crate::resources::ResourceFolder;

pub struct Encoding {}

impl Default for Encoding {
    fn default() -> Self {
        Encoding {}
    }
}

impl Encoding {
    pub fn new() -> Encoding {
        Default::default()
    }

    pub fn frame_from_slice<'a>(
        &self,
        bytes: &'a [u8],
        width: u32,
        height: u32,
    ) -> Result<Frame<'a>, EncodingError> {
        let (dx, dy) = if bytes.len() != (width * height * 3) as usize {
            let real_width = (bytes.len() as u32) / height / 3;
            (real_width, height)
        } else {
            (width, height)
        };
        if bytes.len() != (dx * dy * 3) as usize {
            return Err(EncodingError::InvalidLength);
        }
        match image::ImageBuffer::from_raw(dx, dy, bytes) {
            Some(b) => Ok(b),
            None => Err(EncodingError::InvalidBufferData),
        }
    }

    pub fn export_webm(
        &self,
        folder: &ResourceFolder,
        pattern: &str,
        output: &str,
        framerate: u32,
    ) -> Result<(), EncodingError> {
        let folder = folder.basepath()?;
        invoke_ffmpeg_cli(&folder, pattern, output, framerate)
    }
}

pub mod error {
    use crate::resources::ResourceError;
    use std::error::Error;
    use std::fmt;

    #[derive(Debug)]
    pub enum EncodingError {
        InvalidLength,
        InvalidBufferData,
        FailedToRenderVideo(String),
        InvalidSourceData(String),
    }

    impl fmt::Display for EncodingError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl Error for EncodingError {}

    impl From<ResourceError> for EncodingError {
        fn from(err: ResourceError) -> Self {
            EncodingError::InvalidSourceData(format!("{}", err))
        }
    }
}

#[cfg(test)]
mod test {
    use super::Encoding;
    use std::fs;

    #[test]
    pub fn test_save_as_rgb() {
        let raw_bytes = fs::read("test/data/image.rgb").unwrap();
        let enc = Encoding::new();
        let buffer = enc
            .frame_from_slice(raw_bytes.as_slice(), 3280, 2464)
            .ok()
            .unwrap();
        buffer.save("test/data/image.png").unwrap();
    }
}
