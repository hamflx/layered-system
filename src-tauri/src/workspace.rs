use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use tracing::info;
use uuid::Uuid;

use crate::bcd::{
    bcdedit_boot_sequence, bcdedit_delete, bcdedit_enum_all, bcdedit_set_description,
    extract_guid_for_partition_letter, extract_guid_for_vhd, run_bcdboot,
};
use crate::db::Database;
use crate::diskpart::{
    assign_partitions_script, attach_list_vdisk_script, base_diskpart_script, detach_vdisk_script,
    detail_vdisk_script, diff_attach_list_script, parse_detail_vdisk_parent, parse_list_partition,
    run_diskpart_script,
};
use crate::dism::{apply_image, list_images};
use crate::error::{AppError, Result};
use crate::models::{Node, NodeStatus, WimImageInfo};
use crate::paths::AppPaths;
use crate::state::SharedState;
use crate::sys::{run_elevated_command, CommandOutput};
use crate::temp::TempManager;
use windows_sys::Win32::Storage::FileSystem::GetLogicalDrives;

pub struct WorkspaceService {
    state: SharedState,
}

impl WorkspaceService {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }

    fn db(&self) -> Result<std::sync::Arc<Database>> {
        self.state.db()
    }

    fn paths(&self) -> Result<AppPaths> {
        self.state.paths()
    }

    pub fn scan(&self) -> Result<Vec<Node>> {
        let paths = self.paths()?;
        paths.ensure_layout()?;
        let db = self.db()?;

        let existing_nodes = db.fetch_nodes()?;
        let mut existing_paths: HashMap<String, Node> = existing_nodes
            .iter()
            .map(|n| (normalize_path(&n.path), n.clone()))
            .collect();

        let vhd_paths = collect_vhdx_files(paths.root())?;
        let bcd_enum = if vhd_paths.is_empty() {
            None
        } else {
            bcdedit_enum_all().ok()
        };
        let mut scanned = Vec::new();

        for path in vhd_paths {
            let path_str = path.to_string_lossy().to_string();
            let normalized = normalize_path(&path_str);
            let created_at = file_time_or_now(&path);

            let mut parent_normalized = None;
            let mut detail_ok = true;
            match self.detail_vdisk(&path_str) {
                Ok(detail) => {
                    parent_normalized = detail.parent.map(|p| normalize_path(&p));
                }
                Err(err) => {
                    detail_ok = false;
                    info!("detail_vdisk failed path={} err={err}", path_str);
                }
            }

            let bcd_guid = bcd_enum
                .as_ref()
                .and_then(|out| extract_guid_for_vhd(&out.stdout, &path_str));

            scanned.push(ScannedVhd {
                path: path_str,
                normalized,
                parent_normalized,
                detail_ok,
                created_at,
                bcd_guid,
            });
        }

        // Assign IDs for all discovered VHDX files (reuse existing where possible).
        let mut path_to_id: HashMap<String, String> = existing_paths
            .iter()
            .map(|(p, n)| (p.clone(), n.id.clone()))
            .collect();
        for info in &scanned {
            path_to_id
                .entry(info.normalized.clone())
                .or_insert_with(|| Uuid::new_v4().to_string());
        }

        // Insert newly discovered nodes.
        for info in &scanned {
            if existing_paths.contains_key(&info.normalized) {
                continue;
            }
            let id = path_to_id
                .get(&info.normalized)
                .cloned()
                .expect("id must exist for scanned path");
            let node = Node {
                id: id.clone(),
                parent_id: None,
                name: derive_name_from_path(&info.path),
                path: info.path.clone(),
                bcd_guid: info.bcd_guid.clone(),
                desc: None,
                created_at: info.created_at,
                status: NodeStatus::Normal,
                boot_files_ready: info.bcd_guid.is_some(),
            };
            db.insert_node(&node)?;
            db.insert_op(
                &Uuid::new_v4().to_string(),
                Some(&id),
                "import_vhdx",
                "ok",
                &format!("path={}", node.path),
            )?;
            existing_paths.insert(info.normalized.clone(), node);
        }

        // Update parent linkage and BCD info for existing records.
        for info in &scanned {
            if let Some(node_id) = path_to_id.get(&info.normalized) {
                let target_parent = info
                    .parent_normalized
                    .as_ref()
                    .and_then(|p| path_to_id.get(p).cloned());
                if let Some(existing) = existing_paths.get_mut(&info.normalized) {
                    if existing.parent_id != target_parent {
                        db.update_node_parent(node_id, target_parent.as_deref())?;
                        existing.parent_id = target_parent.clone();
                    }
                    if let Some(guid) = info.bcd_guid.as_ref() {
                        if existing.bcd_guid.as_deref() != Some(guid.as_str()) {
                            db.update_node_bcd(node_id, guid)?;
                            existing.bcd_guid = Some(guid.clone());
                            existing.boot_files_ready = true;
                        }
                    }
                }
            }
        }

        let latest_nodes = db.fetch_nodes()?;
        let detail_lookup: HashMap<String, (Option<String>, bool)> = scanned
            .into_iter()
            .map(|info| (info.normalized, (info.parent_normalized, info.detail_ok)))
            .collect();
        let id_by_path: HashMap<String, String> = latest_nodes
            .iter()
            .map(|n| (normalize_path(&n.path), n.id.clone()))
            .collect();

        for n in latest_nodes.iter() {
            let normalized = normalize_path(&n.path);
            let mut status = NodeStatus::Normal;
            if !Path::new(&n.path).exists() {
                status = NodeStatus::MissingFile;
            } else if let Some((parent_path, detail_ok)) = detail_lookup.get(&normalized) {
                if !detail_ok {
                    status = NodeStatus::Error;
                } else if let Some(parent_norm) = parent_path {
                    match id_by_path.get(parent_norm) {
                        Some(pid) if n.parent_id.as_deref() == Some(pid.as_str()) => {}
                        Some(_) | None => status = NodeStatus::MissingParent,
                    }
                } else if n.parent_id.is_some() {
                    status = NodeStatus::MissingParent;
                }
            }
            db.update_node_status(&n.id, status.clone())?;
            info!("scan node={} status={:?}", n.id, status);
        }

        Ok(db.fetch_nodes()?)
    }

    /// Lightweight fetch without validation; used by UI refresh to avoid slow diskpart checks.
    pub fn list_nodes(&self) -> Result<Vec<Node>> {
        self.db()?.fetch_nodes()
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
        let (efi_letter, sys_letter) = pick_two_letters().ok_or_else(|| {
            AppError::Message("no free drive letter available between S: and Z:".into())
        })?;

        let script = base_diskpart_script(&vhd_path, size_gb, efi_letter, sys_letter);
        let script_path = temp.write_script("create_base.txt", &script)?;
        log_diskpart_script(&script_path);
        let create_res = run_diskpart_script(&script_path)?;
        log_command("diskpart create base", &create_res, Some(&script_path));

        if create_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error(
                "diskpart create base",
                &create_res,
                Some(&script_path),
            ));
        }

        let dism_res = apply_image(wim_file, wim_index, &format!("{sys_letter}:\\"))?;
        log_command("dism apply", &dism_res, None);
        if dism_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("dism apply", &dism_res, None));
        }

        let sys_mount = PathBuf::from(format!("{sys_letter}:"));
        let bcd_res = run_bcdboot(&sys_mount)?;
        log_command("bcdboot", &bcd_res, None);
        if bcd_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("bcdboot", &bcd_res, None));
        }

        let bcd_enum = bcdedit_enum_all()?;
        log_command("bcdedit enum", &bcd_enum, None);
        let guid = extract_guid_for_vhd(&bcd_enum.stdout, vhd_path.to_str().unwrap_or_default())
            .or_else(|| extract_guid_for_partition_letter(&bcd_enum.stdout, sys_letter))
            .unwrap_or_default();

        let detach_script = detach_vdisk_script(&vhd_path, &[efi_letter, sys_letter]);
        let detach_path = temp.write_script("detach_base.txt", &detach_script)?;
        log_diskpart_script(&detach_path);
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
            boot_files_ready: !guid.is_empty(),
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
        let (efi_letter, sys_letter) = pick_two_letters().ok_or_else(|| {
            AppError::Message("no free drive letter available between S: and Z:".into())
        })?;

        let attach_script = diff_attach_list_script(&vhd_path, Path::new(&parent.path));
        let attach_path = temp.write_script("create_diff.txt", &attach_script)?;
        log_diskpart_script(&attach_path);
        let attach_res = run_diskpart_script(&attach_path)?;
        log_command("diskpart create diff", &attach_res, Some(&attach_path));
        if attach_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error(
                "diskpart create diff",
                &attach_res,
                Some(&attach_path),
            ));
        }

        let parts = parse_list_partition(&attach_res.stdout);
        let sys_part = parts
            .iter()
            .find(|p| p.kind.eq_ignore_ascii_case("Primary"))
            .map(|p| p.index)
            .or_else(|| {
                parts
                    .iter()
                    .find(|p| p.kind.eq_ignore_ascii_case("Basic"))
                    .map(|p| p.index)
            });
        let efi_part = parts
            .iter()
            .find(|p| p.kind.eq_ignore_ascii_case("System"))
            .map(|p| p.index)
            .or_else(|| parts.iter().find(|p| p.index == 2).map(|p| p.index));

        let (sys_part, efi_part) = match (sys_part, efi_part) {
            (Some(s), Some(e)) => (s, e),
            _ => {
                return Err(AppError::Message(
                    "failed to detect system/EFI partitions from list partition".into(),
                ))
            }
        };

        let assign_script =
            assign_partitions_script(&vhd_path, &[(efi_part, efi_letter), (sys_part, sys_letter)]);
        let assign_path = temp.write_script("assign_diff.txt", &assign_script)?;
        log_diskpart_script(&assign_path);
        let assign_res = run_diskpart_script(&assign_path)?;
        log_command("diskpart assign diff", &assign_res, Some(&assign_path));
        if assign_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error(
                "diskpart assign diff",
                &assign_res,
                Some(&assign_path),
            ));
        }

        let sys_mount = PathBuf::from(format!("{sys_letter}:"));
        let bcd_res = run_bcdboot(&sys_mount)?;
        log_command("bcdboot", &bcd_res, None);
        if bcd_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("bcdboot", &bcd_res, None));
        }
        let bcd_enum = bcdedit_enum_all()?;
        log_command("bcdedit enum", &bcd_enum, None);
        let guid = extract_guid_for_vhd(&bcd_enum.stdout, vhd_path.to_str().unwrap_or_default())
            .or_else(|| extract_guid_for_partition_letter(&bcd_enum.stdout, sys_letter))
            .unwrap_or_default();

        let detach_script = detach_vdisk_script(&vhd_path, &[efi_letter, sys_letter]);
        let detach_path = temp.write_script("detach_diff.txt", &detach_script)?;
        log_diskpart_script(&detach_path);
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
            boot_files_ready: !guid.is_empty(),
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
                let detach_script = detach_vdisk_script(Path::new(&node.path), &[]);
                let path = temp.write_script("detach_cleanup.txt", &detach_script)?;
                log_diskpart_script(&path);
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

    pub fn delete_bcd(&self, node_id: &str) -> Result<()> {
        let db = self.db()?;
        let node = db
            .fetch_node(node_id)?
            .ok_or_else(|| AppError::Message("node not found".into()))?;
        if let Some(guid) = node.bcd_guid.as_ref() {
            let res = bcdedit_delete(guid)?;
            log_command("bcdedit delete", &res, None);
            if res.exit_code.unwrap_or(-1) != 0 {
                return Err(command_error("bcdedit delete", &res, None));
            }
        }
        db.clear_node_bcd(node_id)?;
        db.insert_op(
            &Uuid::new_v4().to_string(),
            Some(node_id),
            "delete_bcd",
            "ok",
            "",
        )?;
        info!("delete_bcd node={node_id}");
        Ok(())
    }

    pub fn add_bcd_entry(
        &self,
        node_id: &str,
        description: Option<String>,
    ) -> Result<Option<String>> {
        let guid = self.repair_bcd_inner(node_id, description.as_deref())?;
        Ok(guid)
    }

    pub fn update_bcd_description(&self, node_id: &str, description: &str) -> Result<()> {
        let db = self.db()?;
        let node = db
            .fetch_node(node_id)?
            .ok_or_else(|| AppError::Message("node not found".into()))?;
        let guid = node
            .bcd_guid
            .clone()
            .ok_or_else(|| AppError::Message("node missing bcd guid".into()))?;
        let res = bcdedit_set_description(&guid, description)?;
        log_command("bcdedit set description", &res, None);
        if res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("bcdedit set description", &res, None));
        }
        db.insert_op(
            &Uuid::new_v4().to_string(),
            Some(node_id),
            "update_bcd_description",
            "ok",
            description,
        )?;
        Ok(())
    }

    pub fn repair_bcd(&self, node_id: &str) -> Result<Option<String>> {
        self.repair_bcd_inner(node_id, None)
    }

    fn repair_bcd_inner(&self, node_id: &str, description: Option<&str>) -> Result<Option<String>> {
        let db = self.db()?;
        let node = db
            .fetch_node(node_id)?
            .ok_or_else(|| AppError::Message("node not found".into()))?;
        let paths = self.paths()?;
        let temp = TempManager::new(paths.tmp_dir())?;
        let sys_letter = pick_free_letter().ok_or_else(|| {
            AppError::Message("no free drive letter available between S: and Z:".into())
        })?;

        let attach_script = crate::diskpart::attach_list_vdisk_script(Path::new(&node.path));
        let attach_path = temp.write_script("attach_repair.txt", &attach_script)?;
        log_diskpart_script(&attach_path);
        let attach_res = run_diskpart_script(&attach_path)?;
        log_command("diskpart attach repair", &attach_res, Some(&attach_path));
        if attach_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error(
                "diskpart attach",
                &attach_res,
                Some(&attach_path),
            ));
        }

        let parts = parse_list_partition(&attach_res.stdout);
        let sys_part = parts
            .iter()
            .find(|p| p.kind.eq_ignore_ascii_case("Primary"))
            .map(|p| p.index)
            .or_else(|| {
                parts
                    .iter()
                    .find(|p| p.kind.eq_ignore_ascii_case("Basic"))
                    .map(|p| p.index)
            })
            .ok_or_else(|| {
                AppError::Message("failed to detect system partition from list partition".into())
            })?;

        let assign_script =
            assign_partitions_script(Path::new(&node.path), &[(sys_part, sys_letter)]);
        let assign_path = temp.write_script("assign_repair.txt", &assign_script)?;
        log_diskpart_script(&assign_path);
        let assign_res = run_diskpart_script(&assign_path)?;
        log_command("diskpart assign repair", &assign_res, Some(&assign_path));
        if assign_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error(
                "diskpart assign",
                &assign_res,
                Some(&assign_path),
            ));
        }

        let sys_mount = PathBuf::from(format!("{sys_letter}:"));
        let bcd_res = run_bcdboot(&sys_mount)?;
        log_command("bcdboot", &bcd_res, None);
        if bcd_res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("bcdboot", &bcd_res, None));
        }
        let bcd_enum = bcdedit_enum_all()?;
        log_command("bcdedit enum", &bcd_enum, None);
        let guid = extract_guid_for_vhd(&bcd_enum.stdout, &node.path)
            .or_else(|| extract_guid_for_partition_letter(&bcd_enum.stdout, sys_letter));
        if let Some(guid) = &guid {
            db.update_node_bcd(&node.id, guid)?;
            if let Some(desc) = description {
                let res = bcdedit_set_description(guid, desc)?;
                log_command("bcdedit set description", &res, None);
            }
        }

        let detach_script = detach_vdisk_script(Path::new(&node.path), &[sys_letter]);
        let detach_path = temp.write_script("detach_repair.txt", &detach_script)?;
        log_diskpart_script(&detach_path);
        if let Ok(o) = run_diskpart_script(&detach_path) {
            log_command("diskpart detach repair", &o, Some(&detach_path));
        }

        db.insert_op(
            &Uuid::new_v4().to_string(),
            Some(&node.id),
            "repair_bcd",
            "ok",
            description.unwrap_or(""),
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
        log_diskpart_script(&script_path);
        let res = run_diskpart_script(&script_path)?;
        log_command("diskpart detail", &res, Some(&script_path));
        if res.exit_code.unwrap_or(-1) != 0 {
            return Err(command_error("diskpart detail", &res, Some(&script_path)));
        }
        Ok(parse_detail_vdisk_parent(&res.stdout))
    }
}

#[derive(Debug)]
struct ScannedVhd {
    path: String,
    normalized: String,
    parent_normalized: Option<String>,
    detail_ok: bool,
    created_at: DateTime<Utc>,
    bcd_guid: Option<String>,
}

fn collect_vhdx_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("vhdx"))
                .unwrap_or(false)
            {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn normalize_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("\\\\?\\")
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn derive_name_from_path(path: &str) -> String {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("vhdx");
    if let Some((prefix, rest)) = stem.split_once('-') {
        if prefix.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
            return rest.to_string();
        }
    }
    stem.to_string()
}

fn file_time_or_now(path: &Path) -> DateTime<Utc> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.created().or_else(|_| m.modified()).ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now)
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

fn pick_two_letters() -> Option<(char, char)> {
    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        return None;
    }
    let mut free = Vec::new();
    for letter in b'S'..=b'Z' {
        let idx = (letter - b'A') as u32;
        let in_use = (mask & (1 << idx)) != 0;
        if !in_use {
            free.push(letter as char);
        }
        if free.len() >= 2 {
            break;
        }
    }
    if free.len() >= 2 {
        Some((free[0], free[1]))
    } else {
        None
    }
}

fn log_diskpart_script(script: &Path) {
    let mut parts = Vec::new();
    match fs::read_to_string(script) {
        Ok(content) => {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                parts.push(format!("script={trimmed}"));
            }
        }
        Err(err) => parts.push(format!("script_read_err={err}")),
    }
    info!(
        "diskpart script {}: {}",
        script.display(),
        parts.join(" | ")
    );
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
        parts.push(format!("stderr={stderr}"));
    } else if !stdout.is_empty() {
        parts.push(format!("stdout={stdout}"));
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
        parts.push(format!("stderr={stderr}"));
    } else if !stdout.is_empty() {
        parts.push(format!("stdout={stdout}"));
    } else {
        parts.push("no output".into());
    }
    AppError::Message(format!("{name} failed: {}", parts.join(" | ")))
}
