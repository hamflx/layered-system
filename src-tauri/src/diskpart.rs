use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::sys::{run_command, CommandOutput};

#[derive(Debug, Clone)]
pub struct VolumeInfo {
    pub volume: String,
    pub letter: Option<String>,
    pub guid: Option<String>,
    pub label: Option<String>,
    pub fs: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VhdDetail {
    pub parent: Option<String>,
}

/// Run a diskpart script stored at `script_path`.
pub fn run_diskpart_script(script_path: &Path) -> Result<CommandOutput> {
    run_command(
        "diskpart",
        &["/s", script_path.to_string_lossy().as_ref()],
        None,
    )
}

/// Generate script to create and partition a base VHDX with GPT + EFI/MSR/Primary.
pub fn base_diskpart_script(
    vhd_path: &Path,
    size_gb: u64,
    efi_mount: &Path,
    sys_mount: &Path,
) -> String {
    let size_mb = size_gb * 1024;
    format!(
        r#"
create vdisk file="{vhd}" maximum={size_mb} type=expandable
select vdisk file="{vhd}"
attach vdisk
convert gpt
create partition efi size=100
format quick fs=fat32 label="EFI"
assign mount="{efi_mount}"
create partition msr size=16
create partition primary
format quick fs=ntfs label="System"
assign mount="{sys_mount}"
list volume
list partition
"#,
        vhd = vhd_path.display(),
        size_mb = size_mb,
        efi_mount = efi_mount.display(),
        sys_mount = sys_mount.display()
    )
}

/// Script to create a differencing VHDX.
pub fn diff_diskpart_script(
    child: &Path,
    parent: &Path,
    efi_mount: &Path,
    sys_mount: &Path,
) -> String {
    format!(
        r#"
create vdisk file="{child}" parent="{parent}"
select vdisk file="{child}"
attach vdisk
select partition 3
assign mount="{sys_mount}"
select partition 1
assign mount="{efi_mount}"
list volume
list partition
"#,
        child = child.display(),
        parent = parent.display(),
        efi_mount = efi_mount.display(),
        sys_mount = sys_mount.display()
    )
}

pub fn detach_vdisk_script(vhd_path: &Path) -> String {
    format!(
        r#"
select vdisk file="{vhd}"
detach vdisk
"#,
        vhd = vhd_path.display()
    )
}

/// Parse output of `detail vdisk` to extract parent path.
pub fn parse_detail_vdisk_parent(output: &str) -> VhdDetail {
    let mut parent = None;
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("parent path") || lower.contains("parent:") {
            if let Some(idx) = line.find(':') {
                let rest = line[idx + 1..].trim();
                if !rest.is_empty() {
                    parent = Some(rest.to_string());
                }
            }
        }
    }
    VhdDetail { parent }
}

/// Parse `list volume` output to collect volume info.
pub fn parse_list_volume(output: &str) -> Vec<VolumeInfo> {
    let mut volumes = Vec::new();
    for line in output.lines() {
        if line.trim_start().starts_with("Volume ") {
            // Rough parsing: Volume ###  Ltr  Label  Fs ...
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let volume = parts[1].to_string();
                let letter = parts.get(2).map(|s| s.to_string()).filter(|s| s.len() == 1);
                volumes.push(VolumeInfo {
                    volume,
                    letter,
                    guid: None,
                    label: parts.get(3).map(|s| s.to_string()),
                    fs: parts.get(4).map(|s| s.to_string()),
                });
            }
        }
        if line.contains("GUID:") {
            if let Some(last) = volumes.last_mut() {
                if let Some(idx) = line.find("GUID:") {
                    let guid = line[idx + 5..].trim();
                    if !guid.is_empty() {
                        last.guid = Some(guid.to_string());
                    }
                }
            }
        }
    }
    volumes
}

/// Parse `detail vdisk` output to get volumes when attached.
pub fn parse_detail_vdisk_volumes(output: &str) -> Vec<VolumeInfo> {
    parse_list_volume(output)
}

pub fn detail_vdisk_script(vhd_path: &Path) -> String {
    format!(
        r#"
select vdisk file="{vhd}"
detail vdisk
list volume
"#,
        vhd = vhd_path.display()
    )
}
