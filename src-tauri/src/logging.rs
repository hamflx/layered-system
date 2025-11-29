use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::Mutex,
};

use chrono::Utc;

use crate::error::Result;

#[derive(Debug)]
pub struct OpsLogger {
    file: Mutex<std::fs::File>,
}

impl OpsLogger {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    pub fn log_line(&self, action: &str, detail: impl AsRef<str>) -> Result<()> {
        let ts = Utc::now().to_rfc3339();
        let line = format!("{ts} [{action}] {}\n", detail.as_ref());
        let mut guard = self.file.lock().expect("logger mutex poisoned");
        guard.write_all(line.as_bytes())?;
        Ok(())
    }
}
