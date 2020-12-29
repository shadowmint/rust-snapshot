pub use self::error::CaptureError;
use self::helpers::{alloc_frame, as_error, destroy_frame};
use ffmpeg_sys::AVPixelFormat::*;
use ffmpeg_sys::*;
use std::ffi::{c_void, CString};
use std::intrinsics::transmute;
use std::mem::size_of;
use std::os::raw::c_int;
use std::ptr::{null, null_mut};

pub struct CaptureSettings {
    pub backend: String,
    pub device: String,
    pub framerate: u32,
    pub resolution: (u32, u32),
    pub pixel_format: String,
}

impl CaptureSettings {
    fn resolution_as_string(&self) -> String {
        return format!("{}x{}", &self.resolution.0, &self.resolution.1);
    }
}

pub struct Capture {
    pub settings: CaptureSettings,
    context: Option<*mut AVFormatContext>,
    input: Option<*mut AVInputFormat>,
    packet: Option<*mut AVPacket>,
    frame: Option<*mut AVFrame>,
    transcode_frame: Option<*mut AVFrame>,
    codec_context: Option<*mut AVCodecContext>,
    videoindex: i32,
}

impl Capture {
    pub fn new(settings: CaptureSettings) -> Capture {
        Capture {
            settings,
            context: None,
            input: None,
            packet: None,
            frame: None,
            transcode_frame: None,
            codec_context: None,
            videoindex: 0,
        }
    }

    pub fn init(&mut self) -> Result<(), CaptureError> {
        unsafe {
            av_register_all();
            avdevice_register_all();

            // Allocate a libav context
            self.context = Some(avformat_alloc_context());

            // The settings must provide a specific backend; eg. avfoundation, v4l2, etc.
            // libav refers to these as 'input formats', but they're just various backends.
            {
                let backend_name = CString::new(self.settings.backend.as_str())?;
                let input = av_find_input_format(backend_name.as_ptr());
                if input.is_null() {
                    return Err(CaptureError::InvalidDriver);
                }
                self.input = Some(input);
            }

            // If we found a valid backend, attempt to initialize the specified device.
            // This WILL NOT WORK if the settings provided are wrong, and libav offers no
            // way to introspect the devices to determine what settings *are* valid.
            // You should use the ffmpeg cli to find a valid combination of settings for
            // your device and pass them in.
            self.open_device()?;
        }
        Ok(())
    }

    pub fn shutdown(self) {
        if let Some(codec_context) = self.codec_context {
            unsafe {
                avcodec_close(codec_context);
            }
        }
        if let Some(frame) = self.frame {
            unsafe {
                av_free(frame as *mut c_void);
            }
        }
        if let Some(transcode_frame) = self.transcode_frame {
            unsafe {
                // This frame was allocated with a custom buffer; it must be explicitly free'd
                destroy_frame(transcode_frame);
            }
        }
        if let Some(mut packet) = self.packet {
            unsafe {
                av_packet_free(&mut packet);
            }
        }
        if let Some(mut c) = self.context {
            unsafe {
                avformat_close_input(&mut c);
            }
        }
    }

    pub fn get_buffer_size(&self) -> Result<usize, CaptureError> {
        Ok((self.settings.resolution.0 * self.settings.resolution.1 * 3) as usize)
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<(), CaptureError> {
        if self.context.is_none() {
            return Err(CaptureError::NotReady);
        }
        unsafe { self.capture_next_frame(buffer) }
    }

    unsafe fn open_device(&mut self) -> Result<(), CaptureError> {
        let mut context = self.context.unwrap_or(null_mut());
        if context.is_null() {
            return Err(CaptureError::NullPointer(format!("Invalid context")));
        }

        // Every backend takes different options, but these ones are common across (most)
        // implementations. This appears to be the minimum set of configuration required
        // to query an input device. It corresponds directly to the equivlanet call to the
        // cli in the form: ffmpeg -f BACKEND -r FRAMERATE -s RESOLUTION -i DEVICE
        // eg: ffmpeg -f video4linux2 -r 1 -s 640x480 -i "/dev/video0" -frames 1 -y test.png
        let mut device_options: *mut AVDictionary = null_mut();
        {
            let key = CString::new("framerate")?;
            let value = CString::new(format!("{}", self.settings.framerate))?;
            av_dict_set(&mut device_options, key.as_ptr(), value.as_ptr(), 0);
        }
        {
            let key = CString::new("pixel_format")?;
            let value = CString::new(self.settings.pixel_format.as_str())?;
            av_dict_set(&mut device_options, key.as_ptr(), value.as_ptr(), 0);
        }
        {
            let key = CString::new("video_size")?;
            let rez = self.settings.resolution_as_string();
            let value = CString::new(rez.as_str())?;
            av_dict_set(&mut device_options, key.as_ptr(), value.as_ptr(), 0);
        }
        {
            // Attempt to actually open the input device
            let device_name = CString::new(self.settings.device.as_str())?;
            let input = self.input.unwrap_or(null_mut());
            let response = avformat_open_input(
                &mut context,
                device_name.as_ptr(),
                input,
                &mut device_options,
            );
            if response != 0 {
                return Err(as_error(response, "avformat_open_input failed"));
            }
        }

        // Prepare the decoder and find the video stream
        let response = avformat_find_stream_info(context, null_mut());
        if response < 0 {
            return Err(as_error(response, "avformat_find_stream_info failed"));
        }

        let mut videoindex = -1i32;
        let stream_count = (*context).nb_streams;
        for i in 0..stream_count {
            let stream = (*context).streams.offset(i as isize);
            if (*(**stream).codec).codec_type == AVMediaType::AVMEDIA_TYPE_VIDEO {
                videoindex = i as i32;
            }
        }
        if videoindex == -1 {
            return Err(CaptureError::MissingStream(
                "No video streams found in device".to_string(),
            ));
        }
        self.videoindex = videoindex;

        let stream = (*context).streams.offset(videoindex as isize);
        let codec_context = (**stream).codec;
        let codec = avcodec_find_decoder((*codec_context).codec_id);
        if codec.is_null() {
            return Err(CaptureError::MissingCodec(format!(
                "No codec matching {:?} found. avcodec_find_decoder failed",
                (*codec_context).codec_id
            )));
        }

        let response = avcodec_open2(codec_context, codec, null_mut());
        if response < 0 {
            return Err(as_error(response, "avcodec_open2 failed"));
        }

        // The stream is open now!
        self.codec_context = Some(codec_context);

        // Allocate some buffers to use to read and convert data with.
        self.packet = Some(av_malloc(size_of::<AVPacket>()) as *mut AVPacket);
        self.frame = Some(av_frame_alloc());

        Ok(())
    }

    unsafe fn capture_next_frame(&mut self, data: &mut [u8]) -> Result<(), CaptureError> {
        let (context, packet, frame, codec_context) = self.collect_state()?;

        // Loop through, receiving packets until we have an entire frame.
        loop {
            if av_read_frame(context, packet) < 0 {
                continue;
            }

            if (*packet).stream_index != self.videoindex {
                continue;
            }

            let mut got_picture: c_int = 0;
            let response = avcodec_decode_video2(codec_context, frame, &mut got_picture, packet);
            if response < 0 {
                return Err(as_error(response, "avcodec_decode_video2 failed"));
            }

            if got_picture == 0 {
                av_packet_unref(packet);
                continue;
            }

            // So we read some kind of frame in some kind of native format.
            // Now we have to convert that into a standard RGB format to return.
            let fmt = AV_PIX_FMT_RGB24;
            let rgb_frame = self.convert_frame(frame, fmt)?;

            // Now we want to write that into the data buffer we were provided.
            // Yes... this means 3x the image data in memory.
            let buffer_size = av_image_get_buffer_size(fmt, (*frame).width, (*frame).height, 1);
            if data.len() != (buffer_size as usize) {
                return Err(CaptureError::InvalidBuffer(format!(
                    "required size {} != data size {}",
                    buffer_size,
                    data.len()
                )));
            }

            let resp = av_image_copy_to_buffer(
                data.as_mut_ptr(),
                buffer_size,
                transmute(&(*rgb_frame).data[0]),
                transmute(&(*rgb_frame).linesize[0]),
                fmt,
                (*frame).width,
                (*frame).height,
                1,
            );
            if resp < 0 {
                return Err(as_error(response, "av_image_copy_to_buffer failed"));
            }

            av_packet_unref(packet);
            break; // Captured a single frame
        }

        Ok(())
    }

    fn collect_state(
        &mut self,
    ) -> Result<
        (
            *mut AVFormatContext,
            *mut AVPacket,
            *mut AVFrame,
            *mut AVCodecContext,
        ),
        CaptureError,
    > {
        let context = self.context.unwrap_or(null_mut());
        if context.is_null() {
            return Err(CaptureError::NullPointer("Invalid context".to_string()));
        }

        let packet = self.packet.unwrap_or(null_mut());
        if packet.is_null() {
            return Err(CaptureError::NullPointer("Invalid packet".to_string()));
        }

        let codec_context = self.codec_context.unwrap_or(null_mut());
        if codec_context.is_null() {
            return Err(CaptureError::NullPointer(
                "Invalid codec_context".to_string(),
            ));
        }

        let frame = self.frame.unwrap_or(null_mut());
        if frame.is_null() {
            return Err(CaptureError::NullPointer("Invalid frame".to_string()));
        }
        Ok((context, packet, frame, codec_context))
    }

    /// Convert incoming frame (whatever format) to new output frame in target format.
    /// This function allocates a new destination frame of the required shape to use.
    unsafe fn convert_frame(
        &mut self,
        src: *const AVFrame,
        fmt: AVPixelFormat,
    ) -> Result<*mut AVFrame, CaptureError> {
        let in_width = (*src).width;
        let in_height = (*src).height;
        let in_format: AVPixelFormat = std::mem::transmute((*src).format);

        let sws_context = sws_getContext(
            in_width,
            in_height,
            in_format,
            in_width,
            in_height,
            fmt,
            SWS_FAST_BILINEAR,
            null_mut(),
            null_mut(),
            null(),
        );

        let output = match self.transcode_frame {
            Some(frame) => frame,
            None => {
                let frame = alloc_frame(fmt, in_width, in_height);
                self.transcode_frame = Some(frame);
                frame
            }
        };

        // Actually convert pixel data
        sws_scale(
            sws_context,
            transmute(&(*src).data[0]),
            transmute(&(*src).linesize[0]),
            0,
            in_height,
            transmute(&(*output).data[0]),
            transmute(&(*output).linesize[0]),
        );

        Ok(output)
    }
}

mod error {
    use std::ffi::NulError;
    use std::fmt::Formatter;

    #[derive(Debug)]
    pub enum CaptureError {
        NotImplemented,
        InvalidDriver,
        NotReady,
        InvalidBuffer(String),
        MissingStream(String),
        MissingCodec(String),
        NativeError(String),
        NullPointer(String),
    }

    impl std::error::Error for CaptureError {}

    impl std::fmt::Display for CaptureError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl From<NulError> for CaptureError {
        fn from(err: NulError) -> Self {
            CaptureError::NullPointer(format!("{}", err))
        }
    }
}

pub mod helpers {
    use crate::error::CaptureError;
    use ffmpeg_sys::*;
    use image::{ImageBuffer, Rgb};
    use std::ffi::{c_void, CStr};
    use std::mem::transmute;
    use std::os::raw::{c_char, c_int};

    /// Allocates and returns a frame; you must manually destroy it using destroy_frame
    pub unsafe fn alloc_frame(pix_fmt: AVPixelFormat, width: c_int, height: c_int) -> *mut AVFrame {
        let mut frame = av_frame_alloc();
        (*frame).format = transmute(pix_fmt);
        (*frame).width = width;
        (*frame).height = height;
        av_image_alloc(
            transmute(&(*frame).data[0]),
            transmute(&(*frame).linesize[0]),
            width,
            height,
            pix_fmt,
            32,
        );
        frame
    }

    /// Destroy a frame allocated using alloc_frame; don't use this on other frames.
    pub unsafe fn destroy_frame(frame: *mut AVFrame) {
        av_freep(transmute(&(*frame).data[0]));
        av_free(frame as *mut c_void);
    }

    /// Return the libav error detail for an error code
    pub unsafe fn as_error(error_code: c_int, context: &str) -> CaptureError {
        let mut data: Vec<c_char> = Vec::with_capacity(1024);
        av_strerror(error_code, data.as_mut_ptr(), 1024);
        let c_str = CStr::from_ptr(data.as_ptr());
        let msg = match c_str.to_str() {
            Ok(v) => format!("{}: {}", context, v),
            Err(_err) => format!(
                "{}: Failed to get error detail for error code {}",
                context, error_code
            ),
        };
        CaptureError::NativeError(msg)
    }

    /// Convert a byte array to an RgbImage
    pub fn as_rgb_image(
        bytes: &[u8],
        width: u32,
        height: u32,
    ) -> Option<ImageBuffer<Rgb<u8>, &[u8]>> {
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
}

#[cfg(test)]
mod tests {
    use crate::helpers::as_rgb_image;
    use crate::{Capture, CaptureSettings};
    use std::thread::sleep;
    use std::time::Duration;

    #[cfg(target_os = "macos")]
    #[test]
    fn capture_single_frame() {
        let size = (1280, 720);
        let mut capture = Capture::new(CaptureSettings {
            backend: "avfoundation".to_string(),
            device: "0:0".to_string(),
            resolution: size,
            framerate: 24,
            pixel_format: "0rgb".to_string(),
        });

        let buffer_size = capture.get_buffer_size().unwrap();
        let mut buffer = vec![0u8; buffer_size];

        capture.init().unwrap();
        capture.read(buffer.as_mut()).unwrap();
        capture.shutdown();

        let rgb_image = as_rgb_image(buffer.as_slice(), size.0, size.1).unwrap();
        rgb_image.save("snapshot.png").unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn capture_several_frames() {
        let size = (1280, 720);
        let mut capture = Capture::new(CaptureSettings {
            backend: "avfoundation".to_string(),
            device: "0:0".to_string(),
            resolution: size,
            framerate: 24,
            pixel_format: "0rgb".to_string(),
        });

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
