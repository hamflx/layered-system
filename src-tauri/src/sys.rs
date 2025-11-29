use std::path::Path;
use std::process::Command;

use crate::error::{AppError, Result};

#[derive(Debug, serde::Serialize)]
pub struct CommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_command(program: &str, args: &[&str], workdir: Option<&Path>) -> Result<CommandOutput> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = workdir {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .map_err(|e| AppError::Message(format!("Failed to run {program}: {e}")))?;
    Ok(CommandOutput {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}
