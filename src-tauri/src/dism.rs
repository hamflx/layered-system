use crate::error::{AppError, Result};
use crate::models::WimImageInfo;
use crate::sys::{run_command, CommandOutput};

/// List images inside a WIM/ESD file via DISM /Get-WimInfo.
pub fn list_images(image_path: &str) -> Result<Vec<WimImageInfo>> {
    let output = run_command(
        "dism",
        &[
            "/English",
            "/Get-WimInfo",
            &format!("/WimFile:{image_path}"),
        ],
        None,
    )?;
    Ok(parse_wim_info(&output.stdout))
}

/// Apply a WIM/ESD image to a target directory.
pub fn apply_image(image_path: &str, index: u32, apply_dir: &str) -> Result<CommandOutput> {
    run_command(
        "dism",
        &[
            "/English",
            "/Apply-Image",
            &format!("/ImageFile:{image_path}"),
            &format!("/Index:{index}"),
            &format!("/ApplyDir:{apply_dir}"),
        ],
        None,
    )
}

fn parse_wim_info(text: &str) -> Vec<WimImageInfo> {
    let mut result = Vec::new();
    let mut current: Option<WimImageInfo> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Index :") {
            if let Some(info) = current.take() {
                result.push(info);
            }
            if let Some(idx_str) = trimmed.split(':').nth(1) {
                if let Ok(idx) = idx_str.trim().parse::<u32>() {
                    current = Some(WimImageInfo {
                        index: idx,
                        name: String::new(),
                        description: None,
                        size: None,
                    });
                }
            }
        } else if let Some(info) = current.as_mut() {
            if trimmed.starts_with("Name :") {
                if let Some(name) = trimmed.split(':').nth(1) {
                    info.name = name.trim().to_string();
                }
            } else if trimmed.starts_with("Description :") {
                if let Some(desc) = trimmed.split(':').nth(1) {
                    info.description = Some(desc.trim().to_string());
                }
            } else if trimmed.starts_with("Size :") {
                if let Some(sz) = trimmed.split(':').nth(1) {
                    info.size = Some(sz.trim().to_string());
                }
            }
        }
    }
    if let Some(info) = current {
        result.push(info);
    }
    result
}
