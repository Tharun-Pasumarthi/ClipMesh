//! ClipMesh Client Local Storage Engine
//!
//! Handles local SQLite persistence for decrypted clipboard history,
//! offline outbound sync queue, trusted devices cache, and settings.

use chrono::{DateTime, Duration, Utc};
use clipmesh_protocol::{ClipKind, ClipboardPayload, RelayEnvelope};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Item not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredClip {
    pub id: i64,
    pub event_id: Uuid,
    pub kind: ClipKind,
    pub title: String,
    pub body: String,
    pub pinned: bool,
    pub origin_device: String,
    pub created_at: DateTime<Utc>,
}

pub struct LocalStore {
    conn: parking_lot::Mutex<Connection>,
}

impl LocalStore {
    /// Open SQLite database at given path (or in-memory if ":memory:")
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        let store = Self {
            conn: parking_lot::Mutex::new(conn),
        };
        store.run_migrations()?;
        Ok(store)
    }

    /// Open in-memory SQLite store (for tests or ephemeral sessions)
    pub fn in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: parking_lot::Mutex::new(conn),
        };
        store.run_migrations()?;
        Ok(store)
    }

    fn run_migrations(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS local_clips (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id TEXT NOT NULL UNIQUE,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0,
                origin_device TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_local_clips_created ON local_clips(created_at);
            CREATE INDEX IF NOT EXISTS idx_local_clips_pinned ON local_clips(pinned);

            CREATE TABLE IF NOT EXISTS outbound_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                envelope_id TEXT NOT NULL UNIQUE,
                envelope_json TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    /// Insert or update a clipboard item
    pub fn insert_clip(&self, event_id: Uuid, payload: &ClipboardPayload) -> Result<StoredClip, StorageError> {
        let conn = self.conn.lock();
        let now = Utc::now();
        let kind_str = match payload.kind {
            ClipKind::Text => "text",
            ClipKind::Link => "link",
            ClipKind::Code => "code",
        };

        conn.execute(
            r#"
            INSERT INTO local_clips (event_id, kind, title, body, pinned, origin_device, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(event_id) DO UPDATE SET
                body = excluded.body,
                title = excluded.title;
            "#,
            params![
                event_id.to_string(),
                kind_str,
                payload.title,
                payload.text,
                if payload.pinned { 1 } else { 0 },
                payload.origin_device_name,
                now.to_rfc3339()
            ],
        )?;

        let id = conn.last_insert_rowid();
        Ok(StoredClip {
            id,
            event_id,
            kind: payload.kind.clone(),
            title: payload.title.clone(),
            body: payload.text.clone(),
            pinned: payload.pinned,
            origin_device: payload.origin_device_name.clone(),
            created_at: now,
        })
    }

    /// List clips with optional search query and filter
    pub fn list_clips(
        &self,
        query: Option<&str>,
        filter_kind: Option<&str>,
        limit: usize,
    ) -> Result<Vec<StoredClip>, StorageError> {
        let conn = self.conn.lock();
        let mut sql = "SELECT id, event_id, kind, title, body, pinned, origin_device, created_at FROM local_clips WHERE 1=1".to_string();
        let mut param_vals: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(q) = query {
            if !q.is_empty() {
                sql.push_str(" AND (title LIKE ? OR body LIKE ? OR origin_device LIKE ?)");
                let pattern = format!("%{}%", q);
                param_vals.push(Box::new(pattern.clone()));
                param_vals.push(Box::new(pattern.clone()));
                param_vals.push(Box::new(pattern));
            }
        }

        if let Some(k) = filter_kind {
            if k == "pinned" {
                sql.push_str(" AND pinned = 1");
            } else if k != "all" && !k.is_empty() {
                sql.push_str(" AND kind = ?");
                param_vals.push(Box::new(k.to_string()));
            }
        }

        sql.push_str(" ORDER BY pinned DESC, id DESC LIMIT ?");
        param_vals.push(Box::new(limit as i64));

        let params_refs: Vec<&dyn rusqlite::ToSql> = param_vals.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&params_refs[..], |row| {
            let id: i64 = row.get(0)?;
            let event_id_str: String = row.get(1)?;
            let kind_str: String = row.get(2)?;
            let title: String = row.get(3)?;
            let body: String = row.get(4)?;
            let pinned_int: i32 = row.get(5)?;
            let origin_device: String = row.get(6)?;
            let created_at_str: String = row.get(7)?;

            let kind = match kind_str.as_str() {
                "link" => ClipKind::Link,
                "code" => ClipKind::Code,
                _ => ClipKind::Text,
            };

            let event_id = Uuid::parse_str(&event_id_str).unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(StoredClip {
                id,
                event_id,
                kind,
                title,
                body,
                pinned: pinned_int == 1,
                origin_device,
                created_at,
            })
        })?;

        let mut clips = Vec::new();
        for r in rows {
            clips.push(r?);
        }
        Ok(clips)
    }

    /// Toggle pin status
    pub fn toggle_pin(&self, id: i64) -> Result<bool, StorageError> {
        let conn = self.conn.lock();
        let current: Option<i32> = conn
            .query_row("SELECT pinned FROM local_clips WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?;
        if let Some(pinned) = current {
            let next = if pinned == 1 { 0 } else { 1 };
            conn.execute("UPDATE local_clips SET pinned = ?1 WHERE id = ?2", params![next, id])?;
            Ok(next == 1)
        } else {
            Err(StorageError::NotFound(format!("Clip id {}", id)))
        }
    }

    /// Delete clip
    pub fn delete_clip(&self, id: i64) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM local_clips WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Clear all unpinned items
    pub fn clear_unpinned(&self) -> Result<usize, StorageError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM local_clips WHERE pinned = 0", [])?;
        Ok(affected)
    }

    /// Apply retention period cleanup (e.g. 24 hours, 7 days, 30 days)
    pub fn cleanup_retention(&self, days: i64) -> Result<usize, StorageError> {
        if days <= 0 {
            return Ok(0); // Forever retention
        }
        let cutoff = Utc::now() - Duration::days(days);
        let conn = self.conn.lock();
        let affected = conn.execute(
            "DELETE FROM local_clips WHERE pinned = 0 AND created_at < ?1",
            params![cutoff.to_rfc3339()],
        )?;
        Ok(affected)
    }

    /// Outbound queue management: Enqueue envelope
    pub fn enqueue_outbound(&self, envelope: &RelayEnvelope) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let json = serde_json::to_string(envelope)?;
        conn.execute(
            "INSERT INTO outbound_queue (envelope_id, envelope_json, created_at) VALUES (?1, ?2, ?3)",
            params![envelope.id.to_string(), json, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Pop pending outbound envelopes for syncing
    pub fn pending_outbound(&self, limit: usize) -> Result<Vec<(i64, RelayEnvelope)>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, envelope_json FROM outbound_queue ORDER BY id ASC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit as i64], |r| {
            let id: i64 = r.get(0)?;
            let json: String = r.get(1)?;
            Ok((id, json))
        })?;

        let mut envelopes = Vec::new();
        for item in rows {
            let (id, json) = item?;
            if let Ok(env) = serde_json::from_str::<RelayEnvelope>(&json) {
                envelopes.push((id, env));
            }
        }
        Ok(envelopes)
    }

    /// Acknowledge outbound envelope sent
    pub fn ack_outbound(&self, queue_id: i64) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM outbound_queue WHERE id = ?1", params![queue_id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_storage_lifecycle() {
        let store = LocalStore::in_memory().unwrap();
        let payload = ClipboardPayload {
            mime_type: "text/plain".into(),
            kind: ClipKind::Text,
            title: "Test Note".into(),
            text: "Hello from local storage".into(),
            pinned: false,
            origin_device_name: "Windows".into(),
            timestamp_ms: 1000,
        };

        let clip = store.insert_clip(Uuid::new_v4(), &payload).unwrap();
        assert_eq!(clip.body, "Hello from local storage");

        let clips = store.list_clips(None, None, 10).unwrap();
        assert_eq!(clips.len(), 1);

        // Toggle pin
        let is_pinned = store.toggle_pin(clip.id).unwrap();
        assert!(is_pinned);

        // Clear unpinned should not delete pinned item
        let removed = store.clear_unpinned().unwrap();
        assert_eq!(removed, 0);

        store.delete_clip(clip.id).unwrap();
        let after_delete = store.list_clips(None, None, 10).unwrap();
        assert_eq!(after_delete.len(), 0);
    }
}
