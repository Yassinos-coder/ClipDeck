use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;

use super::{ClipboardItem, Shortcut};

/// Thin wrapper around a SQLite connection.
///
/// Designed to be held behind `Arc<Mutex<Storage>>` so it can be shared
/// between the clipboard monitor thread and the GTK main thread.
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Open (or create) the database at the standard data directory.
    pub fn open() -> Result<Self> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create db directory {:?}", parent))?;
        }

        let conn = Connection::open(&path)
            .with_context(|| format!("open SQLite at {:?}", path))?;

        let storage = Self { conn };
        storage.migrate()?;
        Ok(storage)
    }

    /// Path to the SQLite database file.
    pub fn db_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("clipdeck")
            .join("history.db")
    }

    // ── Migrations ────────────────────────────────────────────────────────────

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS clipboard_history (
                 id        INTEGER PRIMARY KEY AUTOINCREMENT,
                 content   TEXT    NOT NULL UNIQUE,
                 timestamp INTEGER NOT NULL,
                 pinned    INTEGER NOT NULL DEFAULT 0
             );

             CREATE INDEX IF NOT EXISTS idx_clipboard_history_timestamp
                 ON clipboard_history (timestamp DESC);

             CREATE TABLE IF NOT EXISTS shortcuts (
                 id      INTEGER PRIMARY KEY AUTOINCREMENT,
                 name    TEXT    NOT NULL,
                 command TEXT    NOT NULL,
                 created INTEGER NOT NULL DEFAULT (strftime('%s','now'))
             );

             CREATE INDEX IF NOT EXISTS idx_shortcuts_created
                 ON shortcuts (created DESC);
            ",
        )?;
        Ok(())
    }

    // ── Clipboard write operations ────────────────────────────────────────────

    /// Insert a new item, returning its assigned `id`.
    ///
    /// If an entry with the same content already exists it is updated
    /// (timestamp refreshed, moved to top of history).
    pub fn insert_item(&mut self, item: &ClipboardItem) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO clipboard_history (content, timestamp, pinned)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(content) DO UPDATE
                 SET timestamp = excluded.timestamp",
            params![item.content, item.timestamp, item.pinned as i32],
        )?;

        let id = self.conn.last_insert_rowid();
        Ok(id)
    }

    /// Toggle the pinned flag for a given row.
    pub fn set_pinned(&mut self, id: i64, pinned: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE clipboard_history SET pinned = ?1 WHERE id = ?2",
            params![pinned as i32, id],
        )?;
        Ok(())
    }

    /// Delete a single entry by id.
    pub fn delete_item(&mut self, id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM clipboard_history WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Remove all non-pinned entries.
    pub fn clear_history(&mut self) -> Result<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM clipboard_history WHERE pinned = 0",
            [],
        )?;
        Ok(deleted)
    }

    /// Enforce the `max_history` cap, evicting oldest unpinned entries.
    pub fn evict_old(&mut self, max: usize) -> Result<()> {
        self.conn.execute(
            "DELETE FROM clipboard_history
             WHERE pinned = 0
               AND id NOT IN (
                   SELECT id FROM clipboard_history
                   ORDER BY timestamp DESC
                   LIMIT ?1
               )",
            params![max as i64],
        )?;
        Ok(())
    }

    // ── Clipboard read operations ─────────────────────────────────────────────

    /// Return at most `limit` most-recent entries, newest first.
    pub fn get_history(&self, limit: usize) -> Result<Vec<ClipboardItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, timestamp, pinned
             FROM clipboard_history
             ORDER BY pinned DESC, timestamp DESC
             LIMIT ?1",
        )?;

        let items = stmt
            .query_map(params![limit as i64], |row| {
                Ok(ClipboardItem {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    timestamp: row.get(2)?,
                    pinned: row.get::<_, i32>(3)? != 0,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(items)
    }

    /// Return whether a content string is already in the database.
    pub fn exists(&self, content: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clipboard_history WHERE content = ?1",
            params![content],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    // ── Shortcut write operations ─────────────────────────────────────────────

    /// Insert a new shortcut, returning its assigned `id`.
    pub fn insert_shortcut(&mut self, name: &str, command: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO shortcuts (name, command, created)
             VALUES (?1, ?2, strftime('%s','now'))",
            params![name, command],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Delete a shortcut by its row id.
    pub fn delete_shortcut(&mut self, id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM shortcuts WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    // ── Shortcut read operations ──────────────────────────────────────────────

    /// Return all shortcuts ordered by creation time (newest first).
    pub fn get_shortcuts(&self) -> Result<Vec<Shortcut>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, command, created
             FROM shortcuts
             ORDER BY created DESC",
        )?;

        let shortcuts = stmt
            .query_map([], |row| {
                Ok(Shortcut {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    command: row.get(2)?,
                    created: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(shortcuts)
    }
}
