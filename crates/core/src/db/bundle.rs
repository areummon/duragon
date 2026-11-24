//! Everything a note owns, taken out of the database or put back in one transaction.
use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::notes::read_note;
use super::threads::{read_chat, read_summary, read_threads, write_summary};
use super::Db;
use crate::model::*;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NoteBundle {
    pub note: Note,
    pub plain: String,
    pub threads: Vec<Thread>,
    pub chat: Vec<Message>,
    pub summary: ChatSummary,
}

fn put_message(tx: &Transaction, note_id: &str, thread_id: Option<&str>, m: &Message) -> Result<()> {
    tx.execute(
        "INSERT INTO messages(id,note_id,thread_id,author,text,drawing,svg,sources,symbols,trimmed,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            m.id, note_id, thread_id, m.author.as_str(), m.text, m.drawing.as_str(), m.svg,
            serde_json::to_string(&m.sources)?, serde_json::to_string(&m.symbols)?, m.trimmed, m.created_at
        ],
    )?;
    Ok(())
}

impl Db {
    /// A transaction that is on disk once this returns, whatever the connection's own `synchronous` is.
    fn durably<T>(&self, work: impl FnOnce(&Transaction) -> Result<T>) -> Result<T> {
        let mut conn = self.conn();
        let before: i64 = conn.pragma_query_value(None, "synchronous", |r| r.get(0))?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        let done = (|| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let out = work(&tx)?;
            tx.commit()?;
            Ok(out)
        })();
        // Staying on FULL only costs speed, so it can't fail work that is already committed.
        let _ = conn.pragma_update(None, "synchronous", before);
        done
    }

    /// Deletes a note with everything it owns, once `keep` has it. An error from `keep` leaves the note. A note that
    /// is not there is already taken, and `keep` is not called.
    pub(crate) fn take_note(&self, id: &str, keep: impl FnOnce(&NoteBundle) -> Result<()>) -> Result<()> {
        self.durably(|tx| {
            let Some(note) = read_note(tx, id)? else { return Ok(()) };
            let bundle = NoteBundle {
                note,
                plain: tx.query_row("SELECT plain FROM notes WHERE id=?1", [id], |r| r.get(0))?,
                threads: read_threads(tx, id)?,
                chat: read_chat(tx, id)?,
                summary: read_summary(tx, id)?,
            };
            keep(&bundle)?;
            tx.execute("DELETE FROM notes WHERE id=?1", [id])?;
            Ok(())
        })
    }

    /// Puts a whole note back. What holds a thread or message decides whose it is. A Mind that is gone becomes the
    /// builtin one, and a folder that is gone none.
    pub(crate) fn put_note(&self, b: &NoteBundle) -> Result<NoteMeta> {
        let id = b.note.meta.id.as_str();
        self.durably(|tx| {
            let n = &b.note;
            tx.execute(
                "INSERT INTO notes(id,title,doc,plain,folder_id,riddler_id,mode,voice_on,web_on,brief,created_at,updated_at)
                 VALUES (?1,?2,?3,?4,(SELECT id FROM folders WHERE id=?5),
                   COALESCE((SELECT id FROM riddlers WHERE id=?6),?7),?8,?9,?10,?11,?12,?13)",
                params![
                    id, n.meta.title, n.doc.to_string(), b.plain, n.meta.folder_id, n.riddler_id, BUILTIN_RIDDLER_ID,
                    n.mode.as_str(), n.voice_on, n.web_on, n.brief, n.created_at, n.meta.updated_at
                ],
            )?;
            for t in &b.threads {
                tx.execute(
                    "INSERT INTO threads(id,note_id,kind,quote,status,pinned,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                    params![t.id, id, t.kind.as_str(), t.quote, t.status.as_str(), t.pinned, t.created_at],
                )?;
                for m in &t.messages {
                    put_message(tx, id, Some(&t.id), m)?;
                }
            }
            for m in &b.chat {
                put_message(tx, id, None, m)?;
            }
            if b.summary != ChatSummary::default() {
                write_summary(tx, id, &b.summary)?;
            }
            read_note(tx, id)?.map(|n| n.meta).ok_or_else(|| anyhow!("note not found: {id}"))
        })
    }
}
