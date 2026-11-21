use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{new_id, now_ms, Db};
use crate::doc;
use crate::model::*;

const MSG_COLS: &str = "id, note_id, thread_id, author, text, drawing, svg, sources, symbols, trimmed, created_at";

fn msg_row(r: &Row) -> rusqlite::Result<Message> {
    let author: String = r.get(3)?;
    let drawing: String = r.get(5)?;
    let sources: String = r.get(7)?;
    let symbols: String = r.get(8)?;
    Ok(Message {
        id: r.get(0)?,
        note_id: r.get(1)?,
        thread_id: r.get(2)?,
        author: Author::parse(&author).unwrap_or(Author::Riddled),
        text: r.get(4)?,
        drawing: DrawingState::parse(&drawing).unwrap_or(DrawingState::None),
        svg: r.get(6)?,
        sources: serde_json::from_str(&sources).unwrap_or_default(),
        symbols: serde_json::from_str(&symbols).unwrap_or_default(),
        trimmed: r.get(9)?,
        created_at: r.get(10)?,
    })
}

fn thread_row(r: &Row) -> rusqlite::Result<Thread> {
    let kind: String = r.get(2)?;
    let status: String = r.get(4)?;
    Ok(Thread {
        id: r.get(0)?,
        note_id: r.get(1)?,
        kind: ThreadKind::parse(&kind).unwrap_or(ThreadKind::Question),
        quote: r.get(3)?,
        status: ThreadStatus::parse(&status).unwrap_or(ThreadStatus::Open),
        pinned: r.get(5)?,
        created_at: r.get(6)?,
        messages: Vec::new(),
    })
}

fn read_summary(conn: &Connection, note_id: &str) -> rusqlite::Result<ChatSummary> {
    conn.query_row(
        "SELECT chat_summary, chat_summarized_through FROM chat_summaries WHERE note_id=?1",
        [note_id],
        |r| Ok(ChatSummary { text: r.get(0)?, through: r.get(1)? }),
    )
    .optional()
    .map(Option::unwrap_or_default)
}

fn write_summary(conn: &Connection, note_id: &str, s: &ChatSummary) -> Result<()> {
    conn.execute(
        "INSERT INTO chat_summaries(note_id, chat_summary, chat_summarized_through) VALUES (?1,?2,?3)
         ON CONFLICT(note_id) DO UPDATE SET chat_summary=excluded.chat_summary,
           chat_summarized_through=excluded.chat_summarized_through",
        params![note_id, s.text, s.through],
    )?;
    Ok(())
}

impl Db {
    pub fn list_threads(&self, note_id: &str) -> Result<Vec<Thread>> {
        let mut threads: Vec<Thread> = {
            let conn = self.conn();
            let mut st = conn.prepare(
                "SELECT id,note_id,kind,quote,status,pinned,created_at FROM threads
                 WHERE note_id=?1 ORDER BY created_at, rowid",
            )?;
            let rows = st.query_map([note_id], thread_row)?.collect::<Result<_, _>>()?;
            rows
        };
        let conn = self.conn();
        let mut st = conn.prepare(&format!(
            "SELECT {MSG_COLS} FROM messages WHERE note_id=?1 AND thread_id IS NOT NULL ORDER BY created_at, rowid"
        ))?;
        for m in st.query_map([note_id], msg_row)? {
            let m = m?;
            if let Some(t) = threads.iter_mut().find(|t| Some(&t.id) == m.thread_id.as_ref()) {
                t.messages.push(m);
            }
        }
        Ok(threads)
    }

    pub fn get_thread(&self, id: &str) -> Result<Thread> {
        let mut t = self
            .conn()
            .query_row(
                "SELECT id,note_id,kind,quote,status,pinned,created_at FROM threads WHERE id=?1",
                [id],
                thread_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("thread not found: {id}"))?;
        let conn = self.conn();
        let mut st = conn.prepare(&format!(
            "SELECT {MSG_COLS} FROM messages WHERE thread_id=?1 ORDER BY created_at, rowid"
        ))?;
        t.messages = st.query_map([id], msg_row)?.collect::<Result<_, _>>()?;
        Ok(t)
    }

    pub fn create_thread(&self, note_id: &str, kind: ThreadKind, quote: &str) -> Result<Thread> {
        let id = new_id();
        self.conn().execute(
            "INSERT INTO threads(id,note_id,kind,quote,status,pinned,created_at) VALUES (?1,?2,?3,?4,'open',0,?5)",
            params![id, note_id, kind.as_str(), quote, now_ms()],
        )?;
        self.get_thread(&id)
    }

    /// `thread_id = None` stores a chat-panel message.
    pub fn add_message(
        &self,
        note_id: &str,
        thread_id: Option<&str>,
        author: Author,
        text: &str,
        drawing: DrawingState,
    ) -> Result<Message> {
        self.add_message_with(note_id, thread_id, author, text, drawing, &[], &[], false)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_message_with(
        &self,
        note_id: &str,
        thread_id: Option<&str>,
        author: Author,
        text: &str,
        drawing: DrawingState,
        sources: &[Source],
        symbols: &[Symbol],
        trimmed: bool,
    ) -> Result<Message> {
        let id = new_id();
        self.conn().execute(
            "INSERT INTO messages(id,note_id,thread_id,author,text,drawing,svg,sources,symbols,trimmed,created_at) VALUES (?1,?2,?3,?4,?5,?6,NULL,?7,?8,?9,?10)",
            params![
                id, note_id, thread_id, author.as_str(), text, drawing.as_str(),
                serde_json::to_string(sources)?, serde_json::to_string(symbols)?, trimmed, now_ms()
            ],
        )?;
        self.get_message(&id)
    }

    pub fn get_message(&self, id: &str) -> Result<Message> {
        self.conn()
            .query_row(
                &format!("SELECT {MSG_COLS} FROM messages WHERE id=?1"),
                [id],
                msg_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("message not found: {id}"))
    }

    pub fn set_message_drawing(&self, id: &str, state: DrawingState, svg: Option<&str>) -> Result<Message> {
        self.conn().execute(
            "UPDATE messages SET drawing=?2, svg=?3 WHERE id=?1",
            params![id, state.as_str(), svg],
        )?;
        self.get_message(id)
    }

    pub fn list_chat(&self, note_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn();
        let mut st = conn.prepare(&format!(
            "SELECT {MSG_COLS} FROM messages WHERE note_id=?1 AND thread_id IS NULL ORDER BY created_at, rowid"
        ))?;
        let rows = st.query_map([note_id], msg_row)?.collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn get_chat_summary(&self, note_id: &str) -> Result<ChatSummary> {
        Ok(read_summary(&self.conn(), note_id)?)
    }

    /// Replaces the summary and its watermark together, only if they are still `expected` and the messages it
    /// covers still exist; false when they moved on or the chat was cleared while it ran.
    pub fn fold_chat_summary(&self, note_id: &str, expected: &ChatSummary, next: &ChatSummary) -> Result<bool> {
        let conn = self.conn();
        if read_summary(&conn, note_id)? != *expected {
            return Ok(false);
        }
        if let Some(through) = &next.through {
            let kept: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM messages WHERE id=?1)", [through], |r| r.get(0))?;
            if !kept {
                return Ok(false);
            }
        }
        write_summary(&conn, note_id, next)?;
        Ok(true)
    }

    /// Deletes the side chat and its summary; margin threads are untouched.
    pub fn clear_chat(&self, note_id: &str) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM messages WHERE note_id=?1 AND thread_id IS NULL", [note_id])?;
        tx.execute("DELETE FROM chat_summaries WHERE note_id=?1", [note_id])?;
        tx.commit()?;
        Ok(())
    }

    /// The writer's own edit: the text changes, the watermark stays.
    pub fn set_chat_summary_text(&self, note_id: &str, text: &str) -> Result<()> {
        let conn = self.conn();
        let through = read_summary(&conn, note_id)?.through;
        write_summary(&conn, note_id, &ChatSummary { text: text.into(), through })
    }

    pub fn set_thread_status(&self, id: &str, status: ThreadStatus) -> Result<()> {
        self.conn()
            .execute("UPDATE threads SET status=?2 WHERE id=?1", params![id, status.as_str()])?;
        Ok(())
    }

    pub fn set_thread_pinned(&self, id: &str, pinned: bool) -> Result<()> {
        self.conn()
            .execute("UPDATE threads SET pinned=?2 WHERE id=?1", params![id, pinned])?;
        Ok(())
    }

    pub fn set_thread_kind(&self, id: &str, kind: ThreadKind) -> Result<()> {
        self.conn()
            .execute("UPDATE threads SET kind=?2 WHERE id=?1", params![id, kind.as_str()])?;
        Ok(())
    }

    /// "Make a note": the quote as a blockquote, then each message prefixed by its author.
    pub fn thread_to_note(&self, id: &str) -> Result<Note> {
        let thread = self.get_thread(id)?;
        let note = self.get_note(&thread.note_id)?;
        let riddler_name = self
            .get_riddler(&note.riddler_id)
            .map(|r| r.name)
            .unwrap_or_else(|_| "Duragon".into());
        let title: String = thread.quote.trim().chars().take(60).collect();
        let mut blocks = vec![doc::title(&title), doc::blockquote(&thread.quote)];
        for m in &thread.messages {
            let who = match m.author {
                Author::You => "You",
                Author::Riddled => riddler_name.as_str(),
            };
            blocks.push(doc::paragraph(&format!("{who}: {}", m.text)));
        }
        self.create_note_with_doc(&title, doc::doc(blocks))
    }
}
