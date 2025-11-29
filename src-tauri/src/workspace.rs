use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
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
use crate::logging::OpsLogger;
use crate::models::{Node, NodeStatus, WimImageInfo};
use crate::paths::AppPaths;
use crate::state::SharedState;
use crate::sys::{run_command, CommandOutput};
use crate::temp::TempManager;

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

    fn logger(&self) -> Option<std::sync::Arc<OpsLogger>> {
        self.state.logger()
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
            if let Some(logger) = self.logger() {
                let _ = logger.log_line("scan", format!("node={} status={:?}", n.id, status));
            }
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
        let efi_mount = paths.mount_root().join("efi");
        let sys_mount = paths.mount_root().join(format!("sys-{id}"));
        fs::create_dir_all(&efi_mount)?;
        fs::create_dir_all(&sys_mount)?;

        let script = base_diskpart_script(&vhd_path, size_gb, &efi_mount, &sys_mount);
        let script_path = temp.write_script("create_base.txt", &script)?;
        let create_res = run_diskpart_script(&script_path)?;

        if create_res.exit_code.unwrap_or(-1) != 0 {
            return Err(AppError::Message(format!(
                "diskpart failed: {}",
                create_res.stderr
            )));
        }

        let dism_res = apply_image(wim_file, wim_index, sys_mount.to_str().unwrap_or_default())?;
        if dism_res.exit_code.unwrap_or(-1) != 0 {
            return Err(AppError::Message(format!(
                "dism failed: {}",
                dism_res.stderr
            )));
        }

        let bcd_res = run_bcdboot(&sys_mount, &efi_mount)?;
        if bcd_res.exit_code.unwrap_or(-1) != 0 {
            return Err(AppError::Message(format!(
                "bcdboot failed: {}",
                bcd_res.stderr
            )));
        }

        let bcd_enum = bcdedit_enum_all()?;
        let guid = extract_guid_for_vhd(&bcd_enum.stdout, vhd_path.to_str().unwrap_or_default())
            .unwrap_or_default();

        let detach_script = detach_vdisk_script(&vhd_path);
        let detach_path = temp.write_script("detach_base.txt", &detach_script)?;
        let _ = run_diskpart_script(&detach_path);

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
        if let Some(logger) = self.logger() {
            let _ = logger.log_line("create_base", format!("id={id} path={}", node.path));
        }
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
        let efi_mount = paths.mount_root().join("efi");
        fs::create_dir_all(&sys_mount)?;
        fs::create_dir_all(&efi_mount)?;

        let script =
            diff_diskpart_script(&vhd_path, Path::new(&parent.path), &efi_mount, &sys_mount);
        let script_path = temp.write_script("create_diff.txt", &script)?;
        let res = run_diskpart_script(&script_path)?;
        if res.exit_code.unwrap_or(-1) != 0 {
            return Err(AppError::Message(format!(
                "diskpart failed: {}",
                res.stderr
            )));
        }

        let bcd_res = run_bcdboot(&sys_mount, &efi_mount)?;
        if bcd_res.exit_code.unwrap_or(-1) != 0 {
            return Err(AppError::Message(format!(
                "bcdboot failed: {}",
                bcd_res.stderr
            )));
        }
        let bcd_enum = bcdedit_enum_all()?;
        let guid = extract_guid_for_vhd(&bcd_enum.stdout, vhd_path.to_str().unwrap_or_default())
            .unwrap_or_default();

        let detach_script = detach_vdisk_script(&vhd_path);
        let detach_path = temp.write_script("detach_diff.txt", &detach_script)?;
        let _ = run_diskpart_script(&detach_path);

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
        if let Some(logger) = self.logger() {
            let _ = logger.log_line("create_diff", format!("id={id} parent={parent_id}"));
        }
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
        db.insert_op(
            &Uuid::new_v4().to_string(),
            Some(node_id),
            "bootsequence_reboot",
            "ok",
            "",
        )?;
        if let Some(logger) = self.logger() {
            let _ = logger.log_line("bootsequence", format!("node={node_id} guid={guid}"));
        }
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
                    let _ = bcdedit_delete(guid);
                }
                // attempt detach
                let temp = TempManager::new(self.paths()?.tmp_dir())?;
                let detach_script = detach_vdisk_script(Path::new(&node.path));
                let path = temp.write_script("detach_cleanup.txt", &detach_script)?;
                let _ = run_diskpart_script(&path);
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
        if let Some(logger) = self.logger() {
            let _ = logger.log_line(
                "delete_subtree",
                format!("node={node_id} count={}", order.len()),
            );
        }
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
        if attach_res.exit_code.unwrap_or(-1) != 0 {
            return Err(AppError::Message(format!(
                "diskpart failed: {}",
                attach_res.stderr
            )));
        }

        let bcd_res = run_bcdboot(&sys_mount, &efi_mount)?;
        if bcd_res.exit_code.unwrap_or(-1) != 0 {
            return Err(AppError::Message(format!(
                "bcdboot failed: {}",
                bcd_res.stderr
            )));
        }
        let bcd_enum = bcdedit_enum_all()?;
        let guid = extract_guid_for_vhd(&bcd_enum.stdout, &node.path);
        if let Some(guid) = &guid {
            db.update_node_bcd(&node.id, guid)?;
        }

        let detach_script = detach_vdisk_script(Path::new(&node.path));
        let detach_path = temp.write_script("detach_repair.txt", &detach_script)?;
        let _ = run_diskpart_script(&detach_path);

        db.insert_op(
            &Uuid::new_v4().to_string(),
            Some(&node.id),
            "repair_bcd",
            "ok",
            "",
        )?;
        if let Some(logger) = self.logger() {
            let _ = logger.log_line(
                "repair_bcd",
                format!("node={} guid={}", node.id, guid.clone().unwrap_or_default()),
            );
        }
        Ok(guid)
    }

    pub fn detail_vdisk(&self, vhd_path: &str) -> Result<crate::diskpart::VhdDetail> {
        let paths = self.paths()?;
        let temp = TempManager::new(paths.tmp_dir())?;
        let script = detail_vdisk_script(Path::new(vhd_path));
        let script_path = temp.write_script("detail_vdisk.txt", &script)?;
        let res = run_diskpart_script(&script_path)?;
        if res.exit_code.unwrap_or(-1) != 0 {
            return Err(AppError::Message(format!(
                "diskpart failed: {}",
                res.stderr
            )));
        }
        Ok(parse_detail_vdisk_parent(&res.stdout))
    }
}

fn bcdedit_boot_sequence_and_reboot(guid: &str) -> Result<CommandOutput> {
    let res = bcdedit_boot_sequence(guid)?;
    // Reboot immediately
    let _ = run_command("shutdown", &["/r", "/t", "0"], None);
    Ok(res)
}
