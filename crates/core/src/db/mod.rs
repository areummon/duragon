//! SQLite storage. `Db` is a cheap clone around one shared connection.
mod misc;
mod notes;
mod threads;

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::doc;
use crate::model::BUILTIN_RIDDLER_ID;

const MIGRATIONS: &[&str] = &[r#"
CREATE TABLE notes(
  id TEXT PRIMARY KEY, title TEXT NOT NULL, doc TEXT NOT NULL, plain TEXT NOT NULL,
  folder_id TEXT, riddler_id TEXT NOT NULL, mode TEXT NOT NULL,
  voice_on INTEGER NOT NULL, web_on INTEGER NOT NULL, brief TEXT NOT NULL,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE VIRTUAL TABLE notes_fts USING fts5(title, plain, content='notes', content_rowid='rowid');
CREATE TRIGGER notes_ai AFTER INSERT ON notes BEGIN
  INSERT INTO notes_fts(rowid, title, plain) VALUES (new.rowid, new.title, new.plain);
END;
CREATE TRIGGER notes_ad AFTER DELETE ON notes BEGIN
  INSERT INTO notes_fts(notes_fts, rowid, title, plain) VALUES ('delete', old.rowid, old.title, old.plain);
END;
CREATE TRIGGER notes_au AFTER UPDATE ON notes BEGIN
  INSERT INTO notes_fts(notes_fts, rowid, title, plain) VALUES ('delete', old.rowid, old.title, old.plain);
  INSERT INTO notes_fts(rowid, title, plain) VALUES (new.rowid, new.title, new.plain);
END;
CREATE TABLE folders(id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at INTEGER NOT NULL);
CREATE TABLE threads(
  id TEXT PRIMARY KEY, note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  kind TEXT NOT NULL, quote TEXT NOT NULL, status TEXT NOT NULL, pinned INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE INDEX threads_note ON threads(note_id);
CREATE TABLE messages(
  id TEXT PRIMARY KEY, note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  thread_id TEXT REFERENCES threads(id) ON DELETE CASCADE, author TEXT NOT NULL,
  text TEXT NOT NULL, drawing TEXT NOT NULL, svg TEXT, created_at INTEGER NOT NULL
);
CREATE INDEX messages_note ON messages(note_id);
CREATE INDEX messages_thread ON messages(thread_id);
CREATE TABLE riddlers(
  id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL, how_it_thinks TEXT NOT NULL,
  tone TEXT NOT NULL, frequency TEXT NOT NULL, builtin INTEGER NOT NULL
);
CREATE TABLE kv(key TEXT PRIMARY KEY, value TEXT NOT NULL);
"#,
r#"ALTER TABLE messages ADD COLUMN sources TEXT NOT NULL DEFAULT '[]';"#,
// Sketches moved to Opus, which recreates Riddled's drawings more faithfully; a stored Sonnet default follows once.
r#"UPDATE kv SET value = json_set(value, '$.drawModel', 'opus')
   WHERE key = 'settings' AND json_extract(value, '$.drawModel') = 'sonnet';"#];

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Self { conn: Arc::new(Mutex::new(conn)) };
        db.migrate()?;
        db.seed()?;
        Ok(db)
    }

    pub(crate) fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn migrate(&self) -> Result<()> {
        let mut conn = self.conn();
        let version: usize = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i + 1)?;
            tx.commit()?;
        }
        Ok(())
    }

    fn seed(&self) -> Result<()> {
        {
            let conn = self.conn();
            // The builtin Mind can't be edited, so the code owns its row and rewrites it on every start.
            conn.execute(
                "INSERT INTO riddlers(id,name,description,how_it_thinks,tone,frequency,builtin)
                 VALUES (?1,'Duragon',?2,?3,'plain','matters',1)
                 ON CONFLICT(id) DO UPDATE SET name=excluded.name, description=excluded.description,
                   how_it_thinks=excluded.how_it_thinks, tone=excluded.tone, frequency=excluded.frequency, builtin=1",
                [
                    BUILTIN_RIDDLER_ID,
                    "Thinks alongside you, and speaks up when it has something worth saying.",
                    "Read along like a curious, careful friend. Ask about what is unclear, unsupported, or worth a second look. Stay quiet when nothing is.",
                ],
            )?;
            let seeded: bool = conn
                .query_row("SELECT 1 FROM kv WHERE key='seeded'", [], |_| Ok(true))
                .unwrap_or(false);
            if seeded {
                return Ok(());
            }
            conn.execute("INSERT INTO kv(key,value) VALUES('seeded','1')", [])?;
            let notes: i64 = conn.query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))?;
            if notes > 0 {
                return Ok(());
            }
        }
        self.create_note_with_doc(
            "Welcome to Duragon",
            doc::doc(vec![
                doc::title("Welcome to Duragon"),
                doc::paragraph("Duragon is a quiet notebook. You write, and Claude reads along. When something is worth a second look, it speaks up in the margin instead of interrupting."),
                doc::paragraph("Want it quieter? Set the note to \"When you ask\" in the row above its title, and it will stay silent until you call on it."),
                doc::paragraph("To ask it something, write \"Duragon, ...\" on a line of its own. Ask it to draw, and it will sketch a diagram right there."),
                doc::paragraph("Photosynthesis happens in the mitochondria."),
            ]),
        )?;
        Ok(())
    }
}
