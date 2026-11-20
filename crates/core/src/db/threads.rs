use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Row};

use super::{new_id, now_ms, Db};
use crate::doc;
use crate::model::*;

const MSG_COLS: &str = "id, note_id, thread_id, author, text, drawing, svg, sources, created_at";

fn msg_row(r: &Row) -> rusqlite::Result<Message> {
    let author: String = r.get(3)?;
    let drawing: String = r.get(5)?;
    let sources: String = r.get(7)?;
    Ok(Message {
        id: r.get(0)?,
        note_id: r.get(1)?,
        thread_id: r.get(2)?,
        author: Author::parse(&author).unwrap_or(Author::Riddled),
        text: r.get(4)?,
        drawing: DrawingState::parse(&drawing).unwrap_or(DrawingState::None),
        svg: r.get(6)?,
        sources: serde_json::from_str(&sources).unwrap_or_default(),
        created_at: r.get(8)?,
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
        self.add_message_with_sources(note_id, thread_id, author, text, drawing, &[])
    }

    pub fn add_message_with_sources(
        &self,
        note_id: &str,
        thread_id: Option<&str>,
        author: Author,
        text: &str,
        drawing: DrawingState,
        sources: &[Source],
    ) -> Result<Message> {
        let id = new_id();
        self.conn().execute(
            "INSERT INTO messages(id,note_id,thread_id,author,text,drawing,svg,sources,created_at) VALUES (?1,?2,?3,?4,?5,?6,NULL,?7,?8)",
            params![id, note_id, thread_id, author.as_str(), text, drawing.as_str(), serde_json::to_string(sources)?, now_ms()],
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
