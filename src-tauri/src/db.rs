use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::{AppError, Result};
use crate::paths::AppPaths;

#[derive(Debug, Clone, Serialize)]
pub struct AppSettings {
    pub root_path: String,
    pub locale: String,
    pub seq_counter: i64,
    pub last_boot_guid: Option<String>,
}

#[derive(Debug)]
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(paths: &AppPaths) -> Result<Self> {
        let conn = Connection::open(paths.state_db_path())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        db.ensure_settings(paths.root())?;
        Ok(db)
    }

    pub fn connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("connection mutex poisoned")
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.connection();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                root_path TEXT NOT NULL DEFAULT '',
                locale TEXT NOT NULL DEFAULT 'zh-CN',
                seq_counter INTEGER NOT NULL DEFAULT 1,
                last_boot_guid TEXT
            );
            INSERT OR IGNORE INTO settings (id, root_path, locale, seq_counter) VALUES (1, '', 'zh-CN', 1);

            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                parent_id TEXT,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                bcd_guid TEXT,
                desc TEXT,
                created_at TEXT NOT NULL,
                status TEXT NOT NULL,
                boot_files_ready INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(parent_id) REFERENCES nodes(id)
            );
            CREATE INDEX IF NOT EXISTS idx_nodes_parent ON nodes(parent_id);

            CREATE TABLE IF NOT EXISTS ops (
                id TEXT PRIMARY KEY,
                node_id TEXT,
                ts TEXT NOT NULL,
                action TEXT NOT NULL,
                result TEXT NOT NULL,
                detail TEXT,
                FOREIGN KEY(node_id) REFERENCES nodes(id)
            );
            "#,
        )?;
        Ok(())
    }

    fn ensure_settings(&self, root: &Path) -> Result<AppSettings> {
        let root_str = root
            .to_str()
            .ok_or_else(|| AppError::Message("Invalid root path".into()))?;
        let mut conn = self.connection();
        conn.execute(
            "UPDATE settings SET root_path = COALESCE(NULLIF(root_path, ''), ?1) WHERE id = 1",
            params![root_str],
        )?;
        drop(conn);
        self.get_settings()
    }

    pub fn update_root_path(&self, root: &Path) -> Result<()> {
        let root_str = root
            .to_str()
            .ok_or_else(|| AppError::Message("Invalid root path".into()))?;
        let mut conn = self.connection();
        conn.execute(
            "UPDATE settings SET root_path = ?1 WHERE id = 1",
            params![root_str],
        )?;
        Ok(())
    }

    pub fn update_locale(&self, locale: &str) -> Result<()> {
        let mut conn = self.connection();
        conn.execute(
            "UPDATE settings SET locale = ?1 WHERE id = 1",
            params![locale],
        )?;
        Ok(())
    }

    pub fn next_seq(&self) -> Result<i64> {
        let mut conn = self.connection();
        conn.execute("UPDATE settings SET seq_counter = seq_counter + 1", [])?;
        let seq: i64 = conn.query_row("SELECT seq_counter FROM settings", [], |row| row.get(0))?;
        Ok(seq)
    }

    pub fn get_settings(&self) -> Result<AppSettings> {
        let conn = self.connection();
        let settings = conn.query_row(
            "SELECT root_path, locale, seq_counter, last_boot_guid FROM settings WHERE id = 1",
            [],
            |row| {
                Ok(AppSettings {
                    root_path: row.get(0)?,
                    locale: row.get(1)?,
                    seq_counter: row.get(2)?,
                    last_boot_guid: row.get(3)?,
                })
            },
        )?;
        Ok(settings)
    }

    pub fn insert_op(
        &self,
        id: &str,
        node_id: Option<&str>,
        action: &str,
        result: &str,
        detail: &str,
    ) -> Result<()> {
        let ts: DateTime<Utc> = Utc::now();
        let mut conn = self.connection();
        conn.execute(
            "INSERT INTO ops (id, node_id, ts, action, result, detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, node_id, ts.to_rfc3339(), action, result, detail],
        )?;
        Ok(())
    }
}
