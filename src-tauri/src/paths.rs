use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn base_dir(&self) -> PathBuf {
        self.root.join("base")
    }

    pub fn diff_dir(&self) -> PathBuf {
        self.root.join("diff")
    }

    pub fn meta_dir(&self) -> PathBuf {
        self.root.join("meta")
    }

    pub fn tmp_dir(&self) -> PathBuf {
        self.meta_dir().join("tmp")
    }

    pub fn locales_dir(&self) -> PathBuf {
        self.meta_dir().join("locales")
    }

    pub fn mount_root(&self) -> PathBuf {
        self.meta_dir().join("mnt")
    }

    pub fn state_db_path(&self) -> PathBuf {
        self.meta_dir().join("state.db")
    }

    pub fn ops_log_path(&self) -> PathBuf {
        self.meta_dir().join("ops.log")
    }

    /// Ensure the expected directory layout exists.
    pub fn ensure_layout(&self) -> Result<()> {
        for dir in [
            self.root(),
            self.base_dir().as_path(),
            self.diff_dir().as_path(),
            self.meta_dir().as_path(),
            self.tmp_dir().as_path(),
            self.locales_dir().as_path(),
            self.mount_root().as_path(),
        ] {
            fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}
