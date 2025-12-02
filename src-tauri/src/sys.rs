use std::path::Path;
use std::process::Command;

use tracing::info;

use crate::error::{AppError, Result};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

fn configure_command_common(
    cmd: &mut Command,
    workdir: Option<&Path>,
) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    if let Some(dir) = workdir {
        cmd.current_dir(dir);
    }
}

pub fn run_command(program: &str, args: &[&str], workdir: Option<&Path>) -> Result<CommandOutput> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    configure_command_common(&mut cmd, workdir);
    let output = cmd
        .output()
        .map_err(|e| AppError::Message(format!("Failed to run {program}: {e}")))?;
    let output = CommandOutput {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    };
    log_command(program, args, workdir, &output);
    Ok(output)
}

pub fn run_elevated_command(
    program: &str,
    args: &[&str],
    workdir: Option<&Path>,
) -> Result<CommandOutput> {
    let output = run_elevated_command_impl(
        program,
        args.iter().map(|s| s.to_string()).collect(),
        workdir,
    )
    .map_err(|err| AppError::Message(err))?;
    log_command(program, args, workdir, &output);
    Ok(output)
}

#[elevated::elevated]
fn run_elevated_command_impl(
    program: &str,
    args: Vec<String>,
    workdir: Option<&Path>,
) -> std::result::Result<CommandOutput, String> {
    let mut cmd = Command::new(program);
    cmd.args(&args);
    configure_command_common(&mut cmd, workdir);
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run {program}: {e}"))?;
    let output = CommandOutput {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    };
    Ok(output)
}

fn log_command(program: &str, args: &[&str], workdir: Option<&Path>, output: &CommandOutput) {
    let mut parts = Vec::new();
    parts.push(format!("cmd={program} {}", args.join(" ")));
    if let Some(dir) = workdir {
        parts.push(format!("cwd={}", dir.display()));
    }
    if let Some(code) = output.exit_code {
        parts.push(format!("exit={code}"));
    }
    let stderr = output.stderr.trim();
    let stdout = output.stdout.trim();
    if !stderr.is_empty() {
        parts.push(format!("stderr={stderr}"));
    } else if !stdout.is_empty() {
        parts.push(format!("stdout={stdout}"));
    }
    info!("{}", parts.join(" | "));
}
