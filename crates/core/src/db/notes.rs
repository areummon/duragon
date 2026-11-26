use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::Value;

use super::revisions::maybe_snapshot_conn;
use super::{new_id, now_ms, Db};
use crate::doc;
use crate::model::*;

const META_COLS: &str = "n.id, n.title, n.folder_id, n.updated_at,
  EXISTS(SELECT 1 FROM threads t WHERE t.note_id=n.id AND t.status='open')";
const NOTE_COLS: &str = "n.id, n.title, n.folder_id, n.updated_at,
  EXISTS(SELECT 1 FROM threads t WHERE t.note_id=n.id AND t.status='open'),
  n.doc, n.riddler_id, n.mode, n.voice_on, n.web_on, n.brief, n.created_at";

pub struct ExportNote {
    pub title: String,
    pub folder_id: Option<String>,
    pub doc: Value,
}

fn meta_row(r: &Row) -> rusqlite::Result<NoteMeta> {
    Ok(NoteMeta {
        id: r.get(0)?,
        title: r.get(1)?,
        folder_id: r.get(2)?,
        updated_at: r.get(3)?,
        has_open_question: r.get(4)?,
    })
}

fn note_row(r: &Row) -> rusqlite::Result<Note> {
    let doc: String = r.get(5)?;
    let mode: String = r.get(7)?;
    Ok(Note {
        meta: meta_row(r)?,
        doc: serde_json::from_str(&doc).unwrap_or(Value::Null),
        riddler_id: r.get(6)?,
        mode: ReadingMode::parse(&mode).unwrap_or(ReadingMode::Thinks),
        voice_on: r.get(8)?,
        web_on: r.get(9)?,
        brief: r.get(10)?,
        created_at: r.get(11)?,
    })
}

pub(super) fn read_note(conn: &Connection, id: &str) -> rusqlite::Result<Option<Note>> {
    conn.prepare_cached(&format!("SELECT {NOTE_COLS} FROM notes n WHERE n.id=?1"))?
        .query_row([id], note_row)
        .optional()
}

fn read_note_meta(conn: &Connection, id: &str) -> rusqlite::Result<Option<NoteMeta>> {
    conn.prepare_cached(&format!("SELECT {META_COLS} FROM notes n WHERE n.id=?1"))?
        .query_row([id], meta_row)
        .optional()
}

pub(super) fn sync_note_links(conn: &Connection, from_note_id: &str, doc_json: &str) -> rusqlite::Result<()> {
    conn.prepare_cached("DELETE FROM note_links WHERE from_note_id=?1")?
        .execute([from_note_id])?;
    if !doc_json.contains("\"noteLink\"") {
        return Ok(());
    }
    let Ok(doc) = serde_json::from_str::<Value>(doc_json) else {
        return Ok(());
    };
    let mut insert = conn.prepare_cached(
        "INSERT OR IGNORE INTO note_links(from_note_id, to_note_id) VALUES (?1, ?2)",
    )?;
    for target in doc::linked_note_ids(&doc) {
        if target != from_note_id {
            insert.execute(params![from_note_id, target])?;
        }
    }
    Ok(())
}

pub(super) fn backfill_note_links(conn: &Connection) -> rusqlite::Result<()> {
    let rows: Vec<(String, String)> = {
        let mut st = conn.prepare("SELECT id, doc FROM notes")?;
        let out = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        out
    };
    let mut insert = conn.prepare(
        "INSERT OR IGNORE INTO note_links(from_note_id, to_note_id) VALUES (?1, ?2)",
    )?;
    for (from_id, doc_json) in rows {
        if !doc_json.contains("\"noteLink\"") {
            continue;
        }
        let Ok(doc) = serde_json::from_str::<Value>(&doc_json) else {
            continue;
        };
        for target in doc::linked_note_ids(&doc) {
            if target != from_id {
                insert.execute(params![from_id, target])?;
            }
        }
    }
    Ok(())
}

/// Quote each token and append `*`, so user input can never be FTS5 syntax.
fn fts_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split_whitespace()
        .map(|t| t.replace('"', ""))
        .filter(|t| t.chars().any(char::is_alphanumeric))
        .map(|t| format!("\"{t}\"*"))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

impl Db {
    pub fn list_notes(&self) -> Result<Vec<NoteMeta>> {
        let conn = self.conn();
        let mut st = conn.prepare_cached(&format!(
            "SELECT {META_COLS} FROM notes n ORDER BY n.updated_at DESC, n.rowid DESC"
        ))?;
        let rows = st.query_map([], meta_row)?.collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn list_notes_for_export(&self) -> Result<Vec<ExportNote>> {
        let conn = self.conn();
        let mut st = conn.prepare_cached(
            "SELECT title, folder_id, doc FROM notes ORDER BY updated_at DESC, rowid DESC",
        )?;
        let rows = st
            .query_map([], |r| {
                let doc: String = r.get(2)?;
                Ok(ExportNote {
                    title: r.get(0)?,
                    folder_id: r.get(1)?,
                    doc: serde_json::from_str(&doc).unwrap_or(Value::Null),
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn search_notes(&self, query: &str) -> Result<Vec<NoteMeta>> {
        let Some(q) = fts_query(query) else {
            return self.list_notes();
        };
        let conn = self.conn();
        let mut st = conn.prepare_cached(&format!(
            "SELECT {META_COLS} FROM notes_fts f JOIN notes n ON n.rowid=f.rowid
             WHERE notes_fts MATCH ?1 ORDER BY rank"
        ))?;
        let rows = st.query_map([q], meta_row)?.collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Notes other than `exclude` matching `query`, best first, each with a short excerpt of its text.
    pub fn search_other_notes(&self, query: &str, exclude: &str, limit: usize) -> Result<Vec<NoteHit>> {
        let Some(q) = fts_query(query) else { return Ok(Vec::new()) };
        let conn = self.conn();
        let mut st = conn.prepare_cached(
            "SELECT n.id, n.title, date(n.updated_at / 1000, 'unixepoch', 'localtime'),
               snippet(notes_fts, 1, '', '', '…', 24)
             FROM notes_fts f JOIN notes n ON n.rowid=f.rowid
             WHERE notes_fts MATCH ?1 AND n.id != ?2 ORDER BY rank LIMIT ?3",
        )?;
        let rows = st
            .query_map(params![q, exclude, limit as i64], |r| {
                Ok(NoteHit { id: r.get(0)?, title: r.get(1)?, edited: r.get(2)?, snippet: r.get(3)? })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Title and plain text of a note, or `None` when there is no such note.
    pub fn note_text(&self, id: &str) -> Result<Option<(String, String)>> {
        Ok(self
            .conn()
            .query_row("SELECT title, plain FROM notes WHERE id=?1", [id], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?)
    }

    /// Notes other than `note_id` whose document contains a `noteLink` referencing `note_id`.
    pub fn list_backlinks(&self, note_id: &str) -> Result<Vec<NoteMeta>> {
        let conn = self.conn();
        let mut st = conn.prepare_cached(&format!(
            "SELECT {META_COLS} FROM note_links l
             JOIN notes n ON n.id = l.from_note_id
             WHERE l.to_note_id = ?1 AND n.id != ?1
             ORDER BY n.updated_at DESC, n.rowid DESC"
        ))?;
        let rows = st.query_map([note_id], meta_row)?.collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn get_note(&self, id: &str) -> Result<Note> {
        self.find_note(id)?.ok_or_else(|| anyhow!("note not found: {id}"))
    }

    pub(crate) fn find_note(&self, id: &str) -> Result<Option<Note>> {
        Ok(read_note(&self.conn(), id)?)
    }

    pub fn create_note(&self, title: Option<&str>) -> Result<Note> {
        let title = title.unwrap_or("").trim();
        self.create_note_with_doc(
            title,
            doc::doc(vec![doc::title(title), doc::paragraph("")]),
        )
    }

    /// Inserts a note, applying the new-note defaults from settings.
    pub fn create_note_with_doc(&self, title: &str, document: Value) -> Result<Note> {
        self.create_note_in_folder(title, document, None)
    }

    /// Inserts a note into an optional folder in a single transaction, applying new-note defaults from settings.
    pub fn create_note_in_folder(&self, title: &str, document: Value, folder_id: Option<&str>) -> Result<Note> {
        let s = self.get_settings()?;
        let id = new_id();
        let t = now_ms();
        let doc_json = document.to_string();
        let plain = doc::plain_text(&document);
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO notes(id,title,doc,plain,folder_id,riddler_id,mode,voice_on,web_on,brief,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'',?10,?10)",
            params![
                id,
                title,
                doc_json,
                plain,
                folder_id,
                BUILTIN_RIDDLER_ID,
                s.new_note_mode.as_str(),
                s.voice_in_new_notes,
                s.web_in_new_notes,
                t
            ],
        )?;
        sync_note_links(&tx, &id, &doc_json)?;
        let note = read_note(&tx, &id)?.ok_or_else(|| anyhow!("note not found: {id}"))?;
        tx.commit()?;
        Ok(note)
    }

    pub fn save_note(&self, id: &str, title: &str, document: &Value, plain: &str) -> Result<NoteMeta> {
        self.save_note_raw(id, title, &document.to_string(), plain)
    }

    /// Saves a note whose `doc` is already serialized JSON, avoiding an unnecessary parse + re-serialize cycle.
    pub fn save_note_raw(&self, id: &str, title: &str, doc_json: &str, plain: &str) -> Result<NoteMeta> {
        let now = now_ms();
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let n = tx
            .prepare_cached("UPDATE notes SET title=?2, doc=?3, plain=?4, updated_at=?5 WHERE id=?1")?
            .execute(params![id, title, doc_json, plain, now])?;
        if n == 0 {
            return Err(anyhow!("note not found: {id}"));
        }
        sync_note_links(&tx, id, doc_json)?;
        if !plain.trim().is_empty() {
            maybe_snapshot_conn(&tx, id, title, doc_json, plain, now)?;
        }
        let meta = read_note_meta(&tx, id)?.ok_or_else(|| anyhow!("note not found: {id}"))?;
        tx.commit()?;
        Ok(meta)
    }

    pub fn update_note_meta(&self, id: &str, patch: &NoteMetaPatch) -> Result<Note> {
        {
            let conn = self.conn();
            let n = conn.execute(
                "UPDATE notes SET
                   riddler_id = COALESCE(?2, riddler_id),
                   mode = COALESCE(?3, mode),
                   voice_on = COALESCE(?4, voice_on),
                   web_on = COALESCE(?5, web_on),
                   brief = COALESCE(?6, brief)
                 WHERE id=?1",
                params![
                    id,
                    patch.riddler_id,
                    patch.mode.map(ReadingMode::as_str),
                    patch.voice_on,
                    patch.web_on,
                    patch.brief
                ],
            )?;
            if n == 0 {
                return Err(anyhow!("note not found: {id}"));
            }
            if let Some(folder) = &patch.folder_id {
                conn.execute("UPDATE notes SET folder_id=?2 WHERE id=?1", params![id, folder])?;
            }
        }
        self.get_note(id)
    }
}
