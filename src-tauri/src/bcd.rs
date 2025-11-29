use std::path::Path;

use crate::error::Result;
use crate::sys::{run_command, CommandOutput};

pub fn run_bcdboot(system_dir: &Path, efi_mount: &Path) -> Result<CommandOutput> {
    let sys_path = system_dir
        .to_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| system_dir.to_string_lossy().to_string());
    let efi_path = efi_mount
        .to_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| efi_mount.to_string_lossy().to_string());
    let sys_arg = format!("{sys_path}\\Windows");
    run_command("bcdboot", &[&sys_arg, "/s", &efi_path, "/f", "UEFI"], None)
}

pub fn bcdedit_enum_all() -> Result<CommandOutput> {
    run_command("bcdedit", &["/enum", "all"], None)
}

pub fn bcdedit_boot_sequence(guid: &str) -> Result<CommandOutput> {
    run_command("bcdedit", &["/bootsequence", guid], None)
}

pub fn bcdedit_delete(guid: &str) -> Result<CommandOutput> {
    run_command("bcdedit", &["/delete", guid], None)
}

/// Extract the identifier (GUID) for an entry whose device path references the given VHD path.
pub fn extract_guid_for_vhd(bcd_output: &str, vhd_path: &str) -> Option<String> {
    let mut current_guid: Option<String> = None;
    let needle = vhd_path.to_ascii_lowercase();
    for line in bcd_output.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("identifier") {
            if let Some(guid) = line.split_whitespace().nth(1) {
                current_guid = Some(guid.trim().to_string());
            }
        }
        if lower.contains("device") || lower.contains("osdevice") {
            if lower.contains("vhd") && lower.contains(&needle) {
                if let Some(guid) = &current_guid {
                    return Some(guid.clone());
                }
            }
        }
    }
    None
}
