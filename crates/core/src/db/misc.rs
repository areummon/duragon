use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Row};

use super::{new_id, now_ms, Db};
use crate::model::*;

fn riddler_row(r: &Row) -> rusqlite::Result<Riddler> {
    let tone: String = r.get(4)?;
    let freq: String = r.get(5)?;
    Ok(Riddler {
        id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        how_it_thinks: r.get(3)?,
        tone: Tone::parse(&tone).unwrap_or(Tone::Plain),
        frequency: Frequency::parse(&freq).unwrap_or(Frequency::Matters),
        builtin: r.get(6)?,
    })
}

const RIDDLER_COLS: &str = "id,name,description,how_it_thinks,tone,frequency,builtin";

impl Db {
    pub fn list_folders(&self) -> Result<Vec<Folder>> {
        let conn = self.conn();
        let mut st = conn.prepare("SELECT id,name,created_at FROM folders ORDER BY name COLLATE NOCASE, rowid")?;
        let rows = st
            .query_map([], |r| Ok(Folder { id: r.get(0)?, name: r.get(1)?, created_at: r.get(2)? }))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn create_folder(&self, name: &str) -> Result<Folder> {
        let f = Folder { id: new_id(), name: name.trim().to_string(), created_at: now_ms() };
        self.conn().execute(
            "INSERT INTO folders(id,name,created_at) VALUES (?1,?2,?3)",
            params![f.id, f.name, f.created_at],
        )?;
        Ok(f)
    }

    pub fn rename_folder(&self, id: &str, name: &str) -> Result<()> {
        self.conn()
            .execute("UPDATE folders SET name=?2 WHERE id=?1", params![id, name.trim()])?;
        Ok(())
    }

    /// Notes in the folder move back to the top level.
    pub fn delete_folder(&self, id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute("UPDATE notes SET folder_id=NULL WHERE folder_id=?1", [id])?;
        conn.execute("DELETE FROM folders WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn get_settings(&self) -> Result<Settings> {
        let raw: Option<String> = self
            .conn()
            .query_row("SELECT value FROM kv WHERE key='settings'", [], |r| r.get(0))
            .optional()?;
        Ok(raw.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        self.conn().execute(
            "INSERT INTO kv(key,value) VALUES('settings',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [serde_json::to_string(settings)?],
        )?;
        Ok(())
    }

    pub fn list_riddlers(&self) -> Result<Vec<Riddler>> {
        let conn = self.conn();
        let mut st = conn.prepare(&format!(
            "SELECT {RIDDLER_COLS} FROM riddlers ORDER BY builtin DESC, name COLLATE NOCASE, rowid"
        ))?;
        let rows = st.query_map([], riddler_row)?.collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn get_riddler(&self, id: &str) -> Result<Riddler> {
        self.conn()
            .query_row(
                &format!("SELECT {RIDDLER_COLS} FROM riddlers WHERE id=?1"),
                [id],
                riddler_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("riddler not found: {id}"))
    }

    /// Inserts or updates a custom Riddler. The builtin one is read-only.
    pub fn save_riddler(&self, r: &Riddler) -> Result<Riddler> {
        let id = if r.id.is_empty() { new_id() } else { r.id.clone() };
        if let Ok(existing) = self.get_riddler(&id) {
            if existing.builtin {
                return Err(anyhow!("the builtin Riddler cannot be edited"));
            }
        }
        self.conn().execute(
            "INSERT INTO riddlers(id,name,description,how_it_thinks,tone,frequency,builtin) VALUES (?1,?2,?3,?4,?5,?6,0)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, description=excluded.description,
               how_it_thinks=excluded.how_it_thinks, tone=excluded.tone, frequency=excluded.frequency",
            params![id, r.name, r.description, r.how_it_thinks, r.tone.as_str(), r.frequency.as_str()],
        )?;
        self.get_riddler(&id)
    }

    /// Notes using the deleted Riddler fall back to the builtin one.
    pub fn delete_riddler(&self, id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute("UPDATE notes SET riddler_id=?2 WHERE riddler_id=?1 AND ?1 IN (SELECT id FROM riddlers WHERE builtin=0)",
            params![id, BUILTIN_RIDDLER_ID])?;
        conn.execute("DELETE FROM riddlers WHERE id=?1 AND builtin=0", [id])?;
        Ok(())
    }
}
