pub use self::error::ResourceError;
use std::fs;
use std::fs::DirEntry;
use std::path::PathBuf;

#[derive(Clone)]
pub struct ResourceFolder {
    ready: bool,
    path: PathBuf,
}

impl ResourceFolder {
    pub fn new(path: &str) -> ResourceFolder {
        ResourceFolder {
            ready: false,
            path: PathBuf::from(path),
        }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn require_existing(mut self) -> Result<ResourceFolder, ResourceError> {
        if !self.exists() {
            return Err(ResourceError::NoSuchFolder(format!("{:?}", self.path)));
        }
        self.ready = true;
        Ok(self)
    }

    pub fn require(mut self) -> Result<ResourceFolder, ResourceError> {
        if !self.exists() {
            std::fs::create_dir_all(&self.path)?;
        }
        self.ready = true;
        Ok(self)
    }

    pub fn path(&self, path: &str) -> Result<PathBuf, ResourceError> {
        if !self.ready {
            return Err(ResourceError::NotReady);
        }
        let mut child_path = self.path.clone();
        child_path.push(path);
        Ok(child_path)
    }

    pub fn basepath(&self) -> Result<PathBuf, ResourceError> {
        if !self.ready {
            return Err(ResourceError::NotReady);
        }
        Ok(self.path.clone())
    }

    /// Note this function enumerates all the files before returning so it can sort the results.
    pub fn enumerate_files(&self) -> Result<Vec<DirEntry>, ResourceError> {
        if !self.ready {
            return Err(ResourceError::NotReady);
        }
        let paths = fs::read_dir(&self.path)?;
        let mut sorted = paths
            .into_iter()
            .filter_map(Result::ok)
            .collect::<Vec<DirEntry>>();
        sorted.sort_by_key(|v| v.file_name());
        Ok(sorted)
    }
}

pub mod error {
    use std::io::Error;

    #[derive(Debug)]
    pub enum ResourceError {
        NotReady,
        NoSuchFolder(String),
        UnableToCreateFolder(String),
    }

    impl std::fmt::Display for ResourceError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl From<std::io::Error> for ResourceError {
        fn from(err: Error) -> Self {
            ResourceError::UnableToCreateFolder(format!("{}", err))
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ResourceError, ResourceFolder};

    #[test]
    pub fn test_resource_folder() -> Result<(), ResourceError> {
        let rf = ResourceFolder::new("test/data");
        rf.require()?;
        Ok(())
    }
}
