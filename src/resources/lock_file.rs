pub use self::error::LockError;
use std::fs;
use std::path::{Path, PathBuf};

/// Tracks an external file to check if a lock is true or not.
/// When the file is removed for any reason, the lock is released.
pub struct LockFile {
    path: PathBuf,
}

impl LockFile {
    pub fn new<T: AsRef<Path>>(path: T) -> LockFile {
        LockFile {
            path: PathBuf::from(path.as_ref()),
        }
    }

    pub fn lock(&self) -> Result<(), LockError> {
        if !self.path.exists() {
            fs::write(&self.path, b"LOCK")?;
        }
        Ok(())
    }

    pub fn is_locked(&self) -> bool {
        self.path.exists()
    }

    pub fn unlock(&self) -> Result<(), LockError> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

pub mod error {
    use std::error::Error;
    use std::fmt;
    use std::fmt::Display;
    use std::io;

    #[derive(Debug)]
    pub enum LockError {
        IoFailed(String),
    }

    impl Display for LockError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl Error for LockError {}

    impl From<io::Error> for LockError {
        fn from(err: io::Error) -> Self {
            LockError::IoFailed(format!("{}", err))
        }
    }
}
