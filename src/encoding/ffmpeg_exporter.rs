use crate::encoding::error::EncodingError;
use std::path::Path;
use std::process::{Command, Stdio};

/// Try to export the frames from input_folder as the given output path.
/// This will only work if the ffmpeg cli is installed and on the path.
/// It should run something like: ffmpeg -framerate 24 -pattern_type glob -i * -c:v libvpx-vp9 -pix_fmt yuva420p -lossless 1 out.webm
pub fn invoke_ffmpeg_cli(
    input_folder: &Path,
    file_pattern: &str,
    output_file: &str,
    framerate: u32,
) -> Result<(), EncodingError> {
    let mut cmd = if cfg!(target_os = "windows") {
        Command::new("ffmpeg.exe")
    } else {
        Command::new("ffmpeg")
    };

    let r = cmd
        .args(&[
            "-y",
            "-framerate",
            &format!("{}", framerate),
            "-pattern_type",
            "glob",
            "-i",
            "*.png",
            "-c:v",
            "libvpx-vp9",
            "-pix_fmt",
            "yuva420p",
            "-lossless",
            "1",
            output_file,
        ])
        .current_dir(&input_folder)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output();

    match r {
        Ok(result) => {
            println!("video encoding status: {}", result.status);
            Ok(())
        }
        Err(err) => Err(EncodingError::FailedToRenderVideo(format!("{}", err))),
    }
}
