use crate::error::RuntimeError;
use rust_snapshot::app::config::Manifest;
use rust_snapshot::app::error::AppError;
use rust_snapshot::app::App;
use rust_snapshot::encoding::Encoding;
use rust_snapshot::resources::ResourceFolder;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::exit;

fn main() -> Result<(), RuntimeError> {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        println!("usage: {} [SETTINGS]", args[0]);
        exit(1);
    }

    let settings = fs::read_to_string(&args[1])?;
    let manifest: Manifest = toml::from_str(settings.as_str())?;

    let encoder = Encoding::new();
    let input = ResourceFolder::new(&manifest.config.output_folder).require_existing()?;
    let full_output = get_full_output_path(&manifest)?;

    encoder.export_webm(
        &input,
        "%d_*",
        &full_output,
        manifest.export.export_framerate,
    )?;

    Ok(())
}

fn get_full_output_path(manifest: &Manifest) -> Result<String, RuntimeError> {
    let mut output = PathBuf::from(&manifest.export.export_file);
    let filename = output
        .file_name()
        .map_or_else(|| None, |v| v.to_str())
        .unwrap_or_else(|| "output.webm")
        .to_string();
    output.pop();
    output = fs::canonicalize(output)?;
    output.push(filename);
    let full_output = output.as_os_str().to_str();
    if full_output.is_none() {
        return Err(RuntimeError::Failed(format!(
            "Unable to resolve '{}' to absolute path",
            &manifest.export.export_file
        )));
    }
    Ok(full_output.unwrap().to_string())
}

mod error {
    use rust_snapshot::app::error::AppError;
    use rust_snapshot::encoding::error::EncodingError;
    use rust_snapshot::resources::ResourceError;
    use std::io;

    #[derive(Debug)]
    pub enum RuntimeError {
        Failed(String),
    }

    impl From<ResourceError> for RuntimeError {
        fn from(err: ResourceError) -> Self {
            RuntimeError::Failed(format!("{}", err))
        }
    }

    impl From<AppError> for RuntimeError {
        fn from(err: AppError) -> Self {
            RuntimeError::Failed(format!("{}", err))
        }
    }

    impl From<io::Error> for RuntimeError {
        fn from(err: io::Error) -> Self {
            RuntimeError::Failed(format!("{}", err))
        }
    }

    impl From<EncodingError> for RuntimeError {
        fn from(err: EncodingError) -> Self {
            RuntimeError::Failed(format!("{}", err))
        }
    }

    impl From<toml::de::Error> for RuntimeError {
        fn from(err: toml::de::Error) -> Self {
            RuntimeError::Failed(format!("invalid manifest: {}", err))
        }
    }
}
