use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use crate::bcd::{
    bcdedit_boot_sequence, bcdedit_delete, bcdedit_enum_all, extract_guid_for_vhd, run_bcdboot,
};
use crate::db::Database;
use crate::diskpart::{
    base_diskpart_script, detach_vdisk_script, detail_vdisk_script, diff_diskpart_script,
    parse_detail_vdisk_parent, run_diskpart_script,
};
use crate::dism::{apply_image, list_images};
use crate::error::{AppError, Result};
use crate::models::{Node, NodeStatus, WimImageInfo};
use crate::paths::AppPaths;
use crate::state::SharedState;
use crate::sys::{run_elevated_command, CommandOutput};
use crate::temp::TempManager;
use windows_sys::Win32::Storage::FileSystem::GetLogicalDrives;

pub struct WorkspaceService<'a> {
    state: &'a SharedState,
}

impl<'a> WorkspaceService<'a> {
    pub fn new(state: &'a SharedState) -> Self {
        Self { state }
    }

    fn db(&self) -> Result<std::sync::Arc<Database>> {
        self.state.db()
    }

    fn paths(&self) -> Result<AppPaths> {
        self.state.paths()
    }

    pub fn scan(&self) -> Result<Vec<Node>> {
        let db = self.db()?;
        let nodes = db.fetch_nodes()?;
        // Basic validations: file existence and parent linkage.
        let mut path_map: HashMap<String, Node> =
            nodes.iter().map(|n| (n.id.clone(), n.clone())).collect();
        for n in nodes.iter() {
            let mut status = n.status.clone();
            if !Path::new(&n.path).exists() {
                status = NodeStatus::MissingFile;
            } else if let Some(parent_id) = &n.parent_id {
                if !path_map.contains_key(parent_id) {
                    status = NodeStatus::MissingParent;
                } else if let Some(detail) = self.detail_vdisk(&n.path).ok() {
                    if let Some(parent_path) = detail.parent {
                        if let Some(parent_node) = path_map.get(parent_id) {
                            if parent_node.path.to_ascii_lowercase()
                                != parent_path.to_ascii_lowercase()
                            {
                                status = NodeStatus::MissingParent;
                            }
                        }
                    }
                }
            }
            db.update_node_status(&n.id, status.clone())?;
            info!("scan node={} status={:?}", n.id, status);
        }
        Ok(db.fetch_nodes()?)
    }

    pub fn list_wim_images(&self, image_path: &str) -> Result<Vec<WimImageInfo>> {
        list_images(image_path)
    }

    pub fn create_base(
        &self,
        name: &str,
        desc: Option<String>,
        wim_file: &str,
        wim_index: u32,
        size_gb: u64,
    ) -> Result<Node> {
        let paths = self.paths()?;
        paths.ensure_layout()?;
        let db = self.db()?;
        let seq = db.next_seq()?;
        let id = Uuid::new_v4().to_string();
        let filename = format!("{seq:04}-{slug}.vhdx", slug = name.to_lowercase());
        let vhd_path = paths.base_dir().join(filename);

        let temp = TempManager::new(paths.tmp_dir())?;
        fs::create_dir_all(paths.mount_root())?;
        let efi_letter = pick_free_letter().ok_or_else(|| {
            AppError::Message("no free drive letter available between S: and Z:".into())
        })?;
        let efi_mount = PathBuf::from(format!("{efi_letter}:"));
        let sys_mount = paths.mount_root().join(format!("sys-{id}"));
        fs::create_dir_all(&sys_mount)?;

        let script = base_diskpart_script(&vhd_path, size_gb, efi_letter, &sys_mount);
        let script_path = temp.write_script("create_base.txt", &script)?;
        let create_res = run_diskpart_script(&script_path)?;
        log_command("diskpart create base", &create_res, Some(&script_path));

        if create_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error(
                "diskpart create base",
                &create_res,
                Some(&script_path),
            ));
        }

        let dism_res = apply_image(wim_file, wim_index, sys_mount.to_str().unwrap_or_default())?;
        log_command("dism apply", &dism_res, None);
        if dism_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("dism apply", &dism_res, None));
        }

        let bcd_res = run_bcdboot(&sys_mount, &efi_mount)?;
        log_command("bcdboot", &bcd_res, None);
        if bcd_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("bcdboot", &bcd_res, None));
        }

        let bcd_enum = bcdedit_enum_all()?;
        log_command("bcdedit enum", &bcd_enum, None);
        let guid = extract_guid_for_vhd(&bcd_enum.stdout, vhd_path.to_str().unwrap_or_default())
            .unwrap_or_default();

        let detach_script = detach_vdisk_script(&vhd_path);
        let detach_path = temp.write_script("detach_base.txt", &detach_script)?;
        let detach_res = run_diskpart_script(&detach_path)?;
        log_command("diskpart detach base", &detach_res, Some(&detach_path));

        let node = Node {
            id: id.clone(),
            parent_id: None,
            name: name.to_string(),
            path: vhd_path.to_string_lossy().to_string(),
            bcd_guid: if guid.is_empty() {
                None
            } else {
                Some(guid.clone())
            },
            desc,
            created_at: Utc::now(),
            status: NodeStatus::Normal,
            boot_files_ready: true,
        };

        db.insert_node(&node)?;
        db.insert_op(
            &Uuid::new_v4().to_string(),
            Some(&id),
            "create_base",
            "ok",
            "",
        )?;
        info!("create_base id={id} path={}", node.path);
        Ok(node)
    }

    pub fn create_diff(&self, parent_id: &str, name: &str, desc: Option<String>) -> Result<Node> {
        let db = self.db()?;
        let parent = db
            .fetch_node(parent_id)?
            .ok_or_else(|| AppError::Message("parent not found".into()))?;
        let paths = self.paths()?;
        paths.ensure_layout()?;
        let seq = db.next_seq()?;
        let id = Uuid::new_v4().to_string();
        let filename = format!("{seq:04}-{slug}.vhdx", slug = name.to_lowercase());
        let vhd_path = paths.diff_dir().join(filename);

        let temp = TempManager::new(paths.tmp_dir())?;
        let sys_mount = paths.mount_root().join(format!("sys-{id}"));
        fs::create_dir_all(&sys_mount)?;

        let efi_letter = pick_free_letter().ok_or_else(|| {
            AppError::Message("no free drive letter available between S: and Z:".into())
        })?;
        let efi_mount = PathBuf::from(format!("{efi_letter}:"));

        let script =
            diff_diskpart_script(&vhd_path, Path::new(&parent.path), efi_letter, &sys_mount);
        let script_path = temp.write_script("create_diff.txt", &script)?;
        let res = run_diskpart_script(&script_path)?;
        log_command("diskpart create diff", &res, Some(&script_path));
        if res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error(
                "diskpart create diff",
                &res,
                Some(&script_path),
            ));
        }

        let bcd_res = run_bcdboot(&sys_mount, &efi_mount)?;
        log_command("bcdboot", &bcd_res, None);
        if bcd_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("bcdboot", &bcd_res, None));
        }
        let bcd_enum = bcdedit_enum_all()?;
        log_command("bcdedit enum", &bcd_enum, None);
        let guid = extract_guid_for_vhd(&bcd_enum.stdout, vhd_path.to_str().unwrap_or_default())
            .unwrap_or_default();

        let detach_script = detach_vdisk_script(&vhd_path);
        let detach_path = temp.write_script("detach_diff.txt", &detach_script)?;
        let detach_res = run_diskpart_script(&detach_path)?;
        log_command("diskpart detach diff", &detach_res, Some(&detach_path));

        let node = Node {
            id: id.clone(),
            parent_id: Some(parent_id.to_string()),
            name: name.to_string(),
            path: vhd_path.to_string_lossy().to_string(),
            bcd_guid: if guid.is_empty() {
                None
            } else {
                Some(guid.clone())
            },
            desc,
            created_at: Utc::now(),
            status: NodeStatus::Normal,
            boot_files_ready: true,
        };
        db.insert_node(&node)?;
        db.insert_op(
            &Uuid::new_v4().to_string(),
            Some(&id),
            "create_diff",
            "ok",
            "",
        )?;
        info!("create_diff id={id} parent={parent_id}");
        Ok(node)
    }

    pub fn set_bootsequence_and_reboot(&self, node_id: &str) -> Result<CommandOutput> {
        let db = self.db()?;
        let node = db
            .fetch_node(node_id)?
            .ok_or_else(|| AppError::Message("node not found".into()))?;
        let guid = node
            .bcd_guid
            .clone()
            .ok_or_else(|| AppError::Message("node missing bcd guid".into()))?;
        let res = bcdedit_boot_sequence_and_reboot(&guid)?;
        log_command("bcdedit bootsequence", &res, None);
        db.insert_op(
            &Uuid::new_v4().to_string(),
            Some(node_id),
            "bootsequence_reboot",
            "ok",
            "",
        )?;
        info!("bootsequence node={node_id} guid={guid}");
        Ok(res)
    }

    pub fn delete_subtree(&self, node_id: &str) -> Result<()> {
        let db = self.db()?;
        let nodes = db.fetch_nodes()?;
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for n in nodes.iter() {
            if let Some(pid) = &n.parent_id {
                graph.entry(pid.clone()).or_default().push(n.id.clone());
            }
        }
        let mut order = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(node_id.to_string());
        while let Some(id) = queue.pop_front() {
            order.push(id.clone());
            if let Some(children) = graph.get(&id) {
                for c in children {
                    queue.push_back(c.clone());
                }
            }
        }
        // Delete children after parents? requirement: delete subtree; we reverse to delete leaves first.
        order.reverse();
        for id in order.iter() {
            if let Some(node) = db.fetch_node(id)?.clone() {
                if let Some(guid) = node.bcd_guid.as_ref() {
                    if let Ok(o) = bcdedit_delete(guid) {
                        log_command("bcdedit delete", &o, None);
                    }
                }
                // attempt detach
                let temp = TempManager::new(self.paths()?.tmp_dir())?;
                let detach_script = detach_vdisk_script(Path::new(&node.path));
                let path = temp.write_script("detach_cleanup.txt", &detach_script)?;
                if let Ok(o) = run_diskpart_script(&path) {
                    log_command("diskpart detach cleanup", &o, Some(&path));
                }
                // delete file
                let _ = fs::remove_file(&node.path);
            }
        }
        db.delete_nodes(&order)?;
        db.insert_op(
            &Uuid::new_v4().to_string(),
            Some(node_id),
            "delete_subtree",
            "ok",
            "",
        )?;
        info!("delete_subtree node={node_id} count={}", order.len());
        Ok(())
    }

    pub fn repair_bcd(&self, node_id: &str) -> Result<Option<String>> {
        let db = self.db()?;
        let node = db
            .fetch_node(node_id)?
            .ok_or_else(|| AppError::Message("node not found".into()))?;
        let paths = self.paths()?;
        let temp = TempManager::new(paths.tmp_dir())?;
        let sys_mount = paths.mount_root().join(format!("sys-{node_id}"));
        let efi_mount = paths.mount_root().join("efi");
        fs::create_dir_all(&sys_mount)?;
        fs::create_dir_all(&efi_mount)?;

        let attach_script = detail_vdisk_script(Path::new(&node.path));
        let attach_path = temp.write_script("attach_repair.txt", &attach_script)?;
        let attach_res = run_diskpart_script(&attach_path)?;
        log_command("diskpart attach repair", &attach_res, Some(&attach_path));
        if attach_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error(
                "diskpart attach",
                &attach_res,
                Some(&attach_path),
            ));
        }

        let bcd_res = run_bcdboot(&sys_mount, &efi_mount)?;
        log_command("bcdboot", &bcd_res, None);
        if bcd_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("bcdboot", &bcd_res, None));
        }
        let bcd_enum = bcdedit_enum_all()?;
        log_command("bcdedit enum", &bcd_enum, None);
        let guid = extract_guid_for_vhd(&bcd_enum.stdout, &node.path);
        if let Some(guid) = &guid {
            db.update_node_bcd(&node.id, guid)?;
        }

        let detach_script = detach_vdisk_script(Path::new(&node.path));
        let detach_path = temp.write_script("detach_repair.txt", &detach_script)?;
        if let Ok(o) = run_diskpart_script(&detach_path) {
            log_command("diskpart detach repair", &o, Some(&detach_path));
        }

        db.insert_op(
            &Uuid::new_v4().to_string(),
            Some(&node.id),
            "repair_bcd",
            "ok",
            "",
        )?;
        info!(
            "repair_bcd node={} guid={}",
            node.id,
            guid.clone().unwrap_or_default()
        );
        Ok(guid)
    }

    pub fn detail_vdisk(&self, vhd_path: &str) -> Result<crate::diskpart::VhdDetail> {
        let paths = self.paths()?;
        let temp = TempManager::new(paths.tmp_dir())?;
        let script = detail_vdisk_script(Path::new(vhd_path));
        let script_path = temp.write_script("detail_vdisk.txt", &script)?;
        let res = run_diskpart_script(&script_path)?;
        log_command("diskpart detail", &res, Some(&script_path));
        if res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("diskpart detail", &res, Some(&script_path)));
        }
        Ok(parse_detail_vdisk_parent(&res.stdout))
    }
}

fn bcdedit_boot_sequence_and_reboot(guid: &str) -> Result<CommandOutput> {
    let res = bcdedit_boot_sequence(guid)?;
    // Reboot immediately
    let _ = run_elevated_command("shutdown", &["/r", "/t", "0"], None);
    Ok(res)
}

fn pick_free_letter() -> Option<char> {
    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        return None;
    }
    for letter in b'S'..=b'Z' {
        let idx = (letter - b'A') as u32;
        let in_use = (mask & (1 << idx)) != 0;
        if !in_use {
            return Some(letter as char);
        }
    }
    None
}

fn log_command(name: &str, output: &CommandOutput, script: Option<&Path>) {
    let mut parts = Vec::new();
    if let Some(code) = output.exit_code {
        parts.push(format!("exit={code}"));
    }
    if let Some(script) = script {
        parts.push(format!("script={}", script.display()));
    }
    let stderr = output.stderr.trim();
    let stdout = output.stdout.trim();
    if !stderr.is_empty() {
        parts.push(format!("stderr={}", truncate(stderr, 800)));
    } else if !stdout.is_empty() {
        parts.push(format!("stdout={}", truncate(stdout, 800)));
    }
    info!("{name}: {}", parts.join(" | "));
}

fn command_error(name: &str, output: &CommandOutput, script: Option<&Path>) -> AppError {
    let mut parts = Vec::new();
    if let Some(code) = output.exit_code {
        parts.push(format!("exit={code}"));
    }
    if let Some(script) = script {
        parts.push(format!("script={}", script.display()));
    }
    let stderr = output.stderr.trim();
    let stdout = output.stdout.trim();
    if !stderr.is_empty() {
        parts.push(format!("stderr={}", truncate(stderr, 800)));
    } else if !stdout.is_empty() {
        parts.push(format!("stdout={}", truncate(stdout, 800)));
    } else {
        parts.push("no output".into());
    }
    AppError::Message(format!("{name} failed: {}", parts.join(" | ")))
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() > max {
        format!("{}...", &text[..max])
    } else {
        text.to_string()
    }
}
