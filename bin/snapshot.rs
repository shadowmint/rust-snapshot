use crate::error::RuntimeError;
use rust_snapshot::app::config::Manifest;
use rust_snapshot::app::App;
use std::fs;
use std::process::exit;

fn main() -> Result<(), RuntimeError> {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        println!("usage: {} [SETTINGS]", args[0]);
        exit(1);
    }

    let settings = fs::read_to_string(&args[1])?;
    let manifest: Manifest = toml::from_str(settings.as_str())?;

    let mut app = App::new(manifest)?;
    app.run()?;

    Ok(())
}

mod error {
    use rust_snapshot::app::error::AppError;
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

    impl From<toml::de::Error> for RuntimeError {
        fn from(err: toml::de::Error) -> Self {
            RuntimeError::Failed(format!("invalid manifest: {}", err))
        }
    }
}
