use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Row};
use serde_json::Value;

use super::{new_id, now_ms, Db};
use crate::doc;
use crate::model::*;

const META_COLS: &str = "n.id, n.title, n.folder_id, n.updated_at,
  EXISTS(SELECT 1 FROM threads t WHERE t.note_id=n.id AND t.status='open')";
const NOTE_COLS: &str = "n.id, n.title, n.folder_id, n.updated_at,
  EXISTS(SELECT 1 FROM threads t WHERE t.note_id=n.id AND t.status='open'),
  n.doc, n.riddler_id, n.mode, n.voice_on, n.web_on, n.brief, n.created_at";

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
        let mut st = conn.prepare(&format!(
            "SELECT {META_COLS} FROM notes n ORDER BY n.updated_at DESC, n.rowid DESC"
        ))?;
        let rows = st.query_map([], meta_row)?.collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn search_notes(&self, query: &str) -> Result<Vec<NoteMeta>> {
        let Some(q) = fts_query(query) else {
            return self.list_notes();
        };
        let conn = self.conn();
        let mut st = conn.prepare(&format!(
            "SELECT {META_COLS} FROM notes_fts f JOIN notes n ON n.rowid=f.rowid
             WHERE notes_fts MATCH ?1 ORDER BY rank"
        ))?;
        let rows = st.query_map([q], meta_row)?.collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn get_note(&self, id: &str) -> Result<Note> {
        self.conn()
            .query_row(
                &format!("SELECT {NOTE_COLS} FROM notes n WHERE n.id=?1"),
                [id],
                note_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("note not found: {id}"))
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
        let s = self.get_settings()?;
        let id = new_id();
        let t = now_ms();
        self.conn().execute(
            "INSERT INTO notes(id,title,doc,plain,folder_id,riddler_id,mode,voice_on,web_on,brief,created_at,updated_at)
             VALUES (?1,?2,?3,?4,NULL,?5,?6,?7,?8,'',?9,?9)",
            params![
                id,
                title,
                document.to_string(),
                doc::plain_text(&document),
                BUILTIN_RIDDLER_ID,
                s.new_note_mode.as_str(),
                s.voice_in_new_notes,
                s.web_in_new_notes,
                t
            ],
        )?;
        self.get_note(&id)
    }

    pub fn save_note(&self, id: &str, title: &str, document: &Value, plain: &str) -> Result<NoteMeta> {
        let n = self.conn().execute(
            "UPDATE notes SET title=?2, doc=?3, plain=?4, updated_at=?5 WHERE id=?1",
            params![id, title, document.to_string(), plain, now_ms()],
        )?;
        if n == 0 {
            return Err(anyhow!("note not found: {id}"));
        }
        Ok(self.get_note(id)?.meta)
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

    pub fn delete_note(&self, id: &str) -> Result<()> {
        self.conn().execute("DELETE FROM notes WHERE id=?1", [id])?;
        Ok(())
    }
}
