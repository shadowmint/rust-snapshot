use image::{ImageBuffer, Rgb};

/// Convert a byte array to an RgbImage
pub fn as_rgb_image(bytes: &[u8], width: u32, height: u32) -> Option<ImageBuffer<Rgb<u8>, &[u8]>> {
    let (dx, dy) = if bytes.len() != (width * height * 3) as usize {
        let real_width = (bytes.len() as u32) / height / 3;
        (real_width, height)
    } else {
        (width, height)
    };
    if bytes.len() != (dx * dy * 3) as usize {
        return None;
    }
    match image::ImageBuffer::from_raw(dx, dy, bytes) {
        Some(b) => Some(b),
        None => None,
    }
}
