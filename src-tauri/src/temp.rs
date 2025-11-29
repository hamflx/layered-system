use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct TempManager {
    base: PathBuf,
}

impl TempManager {
    pub fn new(base: impl Into<PathBuf>) -> Result<Self> {
        let base = base.into();
        fs::create_dir_all(&base)?;
        Ok(Self { base })
    }

    pub fn write_script(&self, name: &str, content: &str) -> Result<PathBuf> {
        let path = self.base.join(name);
        fs::write(&path, content)?;
        Ok(path)
    }

    pub fn cleanup(&self, path: &Path) -> Result<()> {
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
