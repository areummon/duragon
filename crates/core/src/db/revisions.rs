use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::notes::{read_note, sync_note_links};
use super::{new_id, now_ms, Db};
use crate::model::{Note, NoteRevision};

pub const REVISION_INTERVAL_MS: i64 = 180_000;
pub const MAX_REVISIONS_PER_NOTE: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BundledRevision {
    pub id: String,
    pub title: String,
    pub doc_json: String,
    pub plain: String,
    pub created_at: i64,
}

fn latest_revision(conn: &Connection, note_id: &str) -> rusqlite::Result<Option<(String, String, i64)>> {
    conn.prepare_cached(
        "SELECT title, plain, created_at FROM note_revisions
         WHERE note_id=?1 ORDER BY created_at DESC, rowid DESC LIMIT 1",
    )?
    .query_row([note_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
    .optional()
}

fn insert_and_prune(
    conn: &Connection,
    note_id: &str,
    title: &str,
    doc_json: &str,
    plain: &str,
    created_at: i64,
) -> rusqlite::Result<()> {
    let id = new_id();
    conn.prepare_cached(
        "INSERT INTO note_revisions(id, note_id, title, doc_json, plain, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?
    .execute(params![id, note_id, title, doc_json, plain, created_at])?;
    conn.prepare_cached(
        "DELETE FROM note_revisions
         WHERE note_id=?1
           AND rowid NOT IN (
             SELECT rowid FROM note_revisions
             WHERE note_id=?1
             ORDER BY created_at DESC, rowid DESC
             LIMIT ?2
           )",
    )?
    .execute(params![note_id, MAX_REVISIONS_PER_NOTE as i64])?;
    Ok(())
}

pub(super) fn maybe_snapshot_conn(
    conn: &Connection,
    note_id: &str,
    title: &str,
    doc_json: &str,
    plain: &str,
    now: i64,
) -> rusqlite::Result<()> {
    if let Some((last_title, last_plain, last_created_at)) = latest_revision(conn, note_id)? {
        if last_title == title && last_plain == plain {
            return Ok(());
        }
        if last_created_at > now - REVISION_INTERVAL_MS {
            return Ok(());
        }
    }
    insert_and_prune(conn, note_id, title, doc_json, plain, now)
}

pub(super) fn force_snapshot_conn(conn: &Connection, note_id: &str, now: i64) -> Result<()> {
    let (title, doc_json, plain): (String, String, String) = conn
        .prepare_cached("SELECT title, doc, plain FROM notes WHERE id=?1")?
        .query_row([note_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .optional()?
        .ok_or_else(|| anyhow!("note not found: {note_id}"))?;
    if let Some((last_title, last_plain, _)) = latest_revision(conn, note_id)? {
        if last_title == title && last_plain == plain {
            return Ok(());
        }
    }
    insert_and_prune(conn, note_id, &title, &doc_json, &plain, now)?;
    Ok(())
}

pub(super) fn read_revisions_for_bundle(conn: &Connection, note_id: &str) -> rusqlite::Result<Vec<BundledRevision>> {
    let mut st = conn.prepare_cached(
        "SELECT id, title, doc_json, plain, created_at FROM note_revisions
         WHERE note_id=?1 ORDER BY created_at ASC, rowid ASC",
    )?;
    let rows = st
        .query_map([note_id], |r| {
            Ok(BundledRevision {
                id: r.get(0)?,
                title: r.get(1)?,
                doc_json: r.get(2)?,
                plain: r.get(3)?,
                created_at: r.get(4)?,
            })
        })?
        .collect();
    rows
}

pub(super) fn put_revisions(conn: &Connection, note_id: &str, revisions: &[BundledRevision]) -> rusqlite::Result<()> {
    let mut st = conn.prepare_cached(
        "INSERT OR IGNORE INTO note_revisions(id, note_id, title, doc_json, plain, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for r in revisions {
        st.execute(params![r.id, note_id, r.title, r.doc_json, r.plain, r.created_at])?;
    }
    Ok(())
}

impl Db {
    pub fn maybe_snapshot(&self, note_id: &str, title: &str, doc_json: &str, plain: &str, now: i64) -> Result<()> {
        let conn = self.conn();
        maybe_snapshot_conn(&conn, note_id, title, doc_json, plain, now)?;
        Ok(())
    }

    pub fn force_snapshot(&self, note_id: &str) -> Result<()> {
        let conn = self.conn();
        force_snapshot_conn(&conn, note_id, now_ms())
    }

    pub fn list_revisions(&self, note_id: &str) -> Result<Vec<NoteRevision>> {
        let conn = self.conn();
        let mut st = conn.prepare_cached(
            "SELECT id, note_id, title, plain, created_at FROM note_revisions
             WHERE note_id=?1 ORDER BY created_at DESC, rowid DESC",
        )?;
        let rows = st
            .query_map([note_id], |r| {
                Ok(NoteRevision {
                    id: r.get(0)?,
                    note_id: r.get(1)?,
                    title: r.get(2)?,
                    plain: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn restore_revision(&self, note_id: &str, revision_id: &str) -> Result<Note> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let (title, doc_json, plain): (String, String, String) = tx
            .prepare_cached("SELECT title, doc_json, plain FROM note_revisions WHERE id=?1 AND note_id=?2")?
            .query_row(params![revision_id, note_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()?
            .ok_or_else(|| anyhow!("revision not found: {revision_id}"))?;
        let now = now_ms();
        force_snapshot_conn(&tx, note_id, now)?;
        let n = tx
            .prepare_cached("UPDATE notes SET title=?2, doc=?3, plain=?4, updated_at=?5 WHERE id=?1")?
            .execute(params![note_id, title, doc_json, plain, now])?;
        if n == 0 {
            return Err(anyhow!("note not found: {note_id}"));
        }
        sync_note_links(&tx, note_id, &doc_json)?;
        let note = read_note(&tx, note_id)?.ok_or_else(|| anyhow!("note not found: {note_id}"))?;
        tx.commit()?;
        Ok(note)
    }
}
