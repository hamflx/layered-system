use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::sys::{run_elevated_command, CommandOutput};

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

#[derive(Debug, Clone)]
pub struct PartitionInfo {
    pub index: u32,
    pub kind: String,
    pub size_mb: Option<u64>,
}

/// Run a diskpart script stored at `script_path`.
pub fn run_diskpart_script(script_path: &Path) -> Result<CommandOutput> {
    run_elevated_command(
        "diskpart",
        &["/s", script_path.to_string_lossy().as_ref()],
        None,
    )
}

/// Generate script to create and partition a base VHDX with GPT + EFI/MSR/Primary.
pub fn base_diskpart_script(
    vhd_path: &Path,
    size_gb: u64,
    efi_letter: char,
    sys_letter: char,
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
assign letter={efi_letter}
create partition msr size=16
create partition primary
format quick fs=ntfs label="System"
assign letter={sys_letter}
list volume
list partition
"#,
        vhd = vhd_path.display(),
        size_mb = size_mb,
        sys_letter = sys_letter
    )
}

/// Script to create a differencing VHDX and list partitions (no letter assignment).
pub fn diff_attach_list_script(child: &Path, parent: &Path) -> String {
    format!(
        r#"
create vdisk file="{child}" parent="{parent}"
select vdisk file="{child}"
attach vdisk
list volume
list partition
"#,
        child = child.display(),
        parent = parent.display()
    )
}

/// Attach an existing VHD and list its partitions/volumes.
pub fn attach_list_vdisk_script(vhd_path: &Path) -> String {
    format!(
        r#"
select vdisk file="{vhd}"
attach vdisk
list partition
list volume
"#,
        vhd = vhd_path.display()
    )
}

/// Script to assign letters to specific partitions on the currently attached VHD.
pub fn assign_partitions_script(vhd_path: &Path, assignments: &[(u32, char)]) -> String {
    let mut lines = Vec::new();
    lines.push(format!(r#"select vdisk file="{}""#, vhd_path.display()));
    for (part_idx, letter) in assignments {
        lines.push(format!("select partition {part_idx}"));
        lines.push(format!("assign letter={letter} noerr"));
    }
    lines.push("list volume".into());
    lines.join("\n")
}

pub fn detach_vdisk_script(vhd_path: &Path, letters: &[char]) -> String {
    let mut lines = Vec::new();
    let select_vhd = format!(r#"select vdisk file="{}""#, vhd_path.display());
    lines.push(select_vhd.clone());
    for letter in letters {
        lines.push(format!("select volume {letter}"));
        lines.push(format!("remove letter={letter} noerr"));
    }
    lines.push(select_vhd);
    lines.push("detach vdisk".into());
    lines.join("\n")
}

/// Parse output of `detail vdisk` to extract parent path.
pub fn parse_detail_vdisk_parent(output: &str) -> VhdDetail {
    let mut parent = None;
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("parent path")
            || lower.contains("parent:")
            || lower.contains("parent filename")
        {
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

/// Parse `list partition` output.
pub fn parse_list_partition(output: &str) -> Vec<PartitionInfo> {
    let mut parts = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("Partition") {
            let cols: Vec<&str> = trimmed.split_whitespace().collect();
            if cols.len() >= 4 {
                let idx = cols[1].parse::<u32>().unwrap_or(0);
                let kind = cols[2].to_string();
                let mut size_mb = None;
                for w in cols.iter().rev() {
                    if let Some(val) = parse_size_mb(w) {
                        size_mb = Some(val);
                        break;
                    }
                }
                parts.push(PartitionInfo {
                    index: idx,
                    kind,
                    size_mb,
                });
            }
        }
    }
    parts
}

fn parse_size_mb(token: &str) -> Option<u64> {
    let lower = token.to_ascii_lowercase();
    if lower.ends_with("mb") {
        let num = lower.trim_end_matches("mb").trim();
        return num.parse::<u64>().ok();
    }
    if lower.ends_with("gb") {
        let num = lower.trim_end_matches("gb").trim();
        if let Ok(val) = num.parse::<u64>() {
            return Some(val * 1024);
        }
    }
    None
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
