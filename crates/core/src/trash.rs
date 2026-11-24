//! Deleted notes. Each is one file in `.trash/`, and the database wins: a file whose note is live is hidden, so a
//! delete or restore that stops halfway neither loses the note nor shows it twice.
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{ErrorKind, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, ensure, Context, Result};
use serde::{Deserialize, Serialize};

use crate::ai::{svg, tasks::clean_sources};
use crate::db::{now_ms, Db, NoteBundle};
use crate::model::*;
use crate::private;

/// Raise it when a file this code can't read would be written; older versions then leave that file alone.
const FORMAT: u32 = 1;
const TITLE_MAX_CHARS: usize = 60;
/// Leaves room in a 255 byte name for the id, the extension and what the system trash adds.
const TITLE_MAX_BYTES: usize = 180;
const UUID_LEN: usize = 36;

/// The operating system's own trash.
pub trait SystemTrash: Send + Sync {
    /// Moves `file` there. Whether the file is still in place says whether it went, not the result.
    fn take(&self, file: &Path) -> Result<()>;
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrashFile<B> {
    duragon_trash: u32,
    deleted_at: i64,
    #[serde(flatten)]
    bundle: B,
}

/// A note in the trash, and the file that holds it.
struct Kept {
    id: NoteId,
    path: PathBuf,
    file: TrashFile<NoteBundle>,
}

fn parse(bytes: &[u8], id: &NoteId) -> Result<TrashFile<NoteBundle>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Version {
        duragon_trash: u32,
    }
    let Version { duragon_trash } = serde_json::from_slice(bytes)?;
    ensure!(duragon_trash == FORMAT, "trash file version {duragon_trash}");
    let file: TrashFile<NoteBundle> = serde_json::from_slice(bytes)?;
    let note = &file.bundle.note;
    ensure!(note.meta.id == id.to_string(), "the file holds another note");
    ensure!(note.doc["type"] == "doc" && note.doc["content"].is_array(), "the note has no document");
    Ok(file)
}

/// The id a file's name ends in.
fn named(path: &Path) -> Option<NoteId> {
    let stem = path.file_name()?.to_str()?.strip_suffix(".json")?;
    let at = stem.len().checked_sub(UUID_LEN)?;
    stem.get(at..)?.parse().ok().filter(|_| stem[..at].ends_with(' '))
}

fn open(path: PathBuf) -> Result<Kept> {
    let id = named(&path).ok_or_else(|| anyhow!("not named like a note"))?;
    ensure!(fs::symlink_metadata(&path)?.is_file(), "not a file");
    let file = parse(&fs::read(&path)?, &id)?;
    Ok(Kept { id, path, file })
}

/// The title as it can appear in a file name on any system.
fn safe_title(title: &str) -> String {
    let plain: String = title
        .chars()
        .map(|c| if c.is_control() || r#"/\:*?"<>|"#.contains(c) { ' ' } else { c })
        .collect();
    let plain = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    for c in plain.trim_start_matches(['.', ' ']).chars().take(TITLE_MAX_CHARS) {
        if out.len() + c.len_utf8() > TITLE_MAX_BYTES {
            break;
        }
        out.push(c);
    }
    let out = out.trim_end();
    if out.is_empty() { "Untitled".into() } else { out.into() }
}

/// Dates a query or the page works out fail on a time too far off.
fn sane(ms: i64) -> i64 {
    ms.clamp(0, now_ms())
}

/// What a file says is not trusted: a drawing is markup the page runs, and a source is a link it opens.
fn clean(m: &mut Message) {
    m.created_at = sane(m.created_at);
    (m.drawing, m.svg) = match (m.drawing, m.svg.take().map(|s| svg::sanitize(&s))) {
        (DrawingState::Done, Some(Ok(svg))) => (DrawingState::Done, Some(svg)),
        (DrawingState::None, _) => (DrawingState::None, None),
        // Nothing is drawing it any more.
        (DrawingState::Pending | DrawingState::Done | DrawingState::Failed, _) => (DrawingState::Failed, None),
    };
    m.sources = clean_sources(&m.sources);
}

fn cleaned(mut bundle: NoteBundle) -> NoteBundle {
    let note = &mut bundle.note;
    note.created_at = sane(note.created_at);
    note.meta.updated_at = sane(note.meta.updated_at);
    for t in &mut bundle.threads {
        t.created_at = sane(t.created_at);
    }
    bundle.threads.iter_mut().flat_map(|t| &mut t.messages).chain(&mut bundle.chat).for_each(clean);
    bundle.summary.text = bundle.summary.text.chars().take(SUMMARY_MAX_CHARS).collect();
    bundle
}

fn unlink(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(e).with_context(|| format!("removing {}", path.display())),
        _ => Ok(()),
    }
}

#[derive(Clone)]
pub struct Trash {
    db: Db,
    dir: PathBuf,
    os: Arc<dyn SystemTrash>,
}

impl Trash {
    pub fn new(db: Db, dir: PathBuf, os: Arc<dyn SystemTrash>) -> Self {
        Self { db, dir, os }
    }

    /// One operation at a time, across windows and processes. Without it, a restore could remove the file a second
    /// delete had just written.
    fn lock(&self) -> Result<File> {
        private::dir(&self.dir)?;
        let lock = private::options().create(true).truncate(false).open(self.dir.join(".lock"))?;
        lock.lock()?;
        Ok(lock)
    }

    fn files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "json") {
                files.push(path);
            }
        }
        Ok(files)
    }

    /// The readable copies of one note, newest first. More than one when an old copy came back from the system trash.
    fn copies(&self, id: &NoteId) -> Result<Vec<Kept>> {
        let mut copies: Vec<Kept> = self
            .files()?
            .into_iter()
            .filter(|p| named(p).as_ref() == Some(id))
            .filter_map(|p| open(p).ok())
            .collect();
        copies.sort_by_key(|c| std::cmp::Reverse(c.file.deleted_at));
        Ok(copies)
    }

    fn tmp(&self, id: &NoteId) -> PathBuf {
        self.dir.join(format!(".{id}.tmp"))
    }

    /// Removes every copy of a note, and what an interrupted write left of it.
    fn remove(&self, id: &NoteId) -> Result<()> {
        unlink(&self.tmp(id))?;
        for c in self.copies(id)? {
            unlink(&c.path)?;
        }
        self.sync_dir()
    }

    #[cfg(unix)]
    fn sync_dir(&self) -> Result<()> {
        Ok(File::open(&self.dir)?.sync_all()?)
    }
    #[cfg(not(unix))]
    fn sync_dir(&self) -> Result<()> {
        Ok(())
    }

    /// A name no other file has, so a file Duragon can't read is never written over.
    fn free_path(&self, id: &NoteId, title: &str) -> PathBuf {
        let title = safe_title(title);
        let plain = self.dir.join(format!("{title} {id}.json"));
        std::iter::once(plain)
            .chain((2..).map(|n| self.dir.join(format!("{title} ({n}) {id}.json"))))
            .find(|p| fs::symlink_metadata(p).is_err())
            .expect("the names never run out")
    }

    fn write(&self, id: &NoteId, bundle: &NoteBundle) -> Result<PathBuf> {
        let bytes = serde_json::to_vec_pretty(&TrashFile { duragon_trash: FORMAT, deleted_at: now_ms(), bundle })?;
        parse(&bytes, id).context("the note could not be read back")?;

        self.remove(id)?;
        let tmp = self.tmp(id);
        let mut f = private::options().create_new(true).open(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
        drop(f);
        let path = self.free_path(id, &bundle.note.meta.title);
        fs::rename(&tmp, &path)?;
        self.sync_dir()?;
        Ok(path)
    }

    /// Whether the system trash now has the file. Its code may fail or panic before or after moving it.
    fn hand_over(&self, path: &Path) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| self.os.take(path))).unwrap_or_else(|_| Err(anyhow!("it panicked")));
        let gone = fs::symlink_metadata(path).is_err_and(|e| e.kind() == ErrorKind::NotFound);
        if let (false, Err(e)) = (gone, result) {
            eprintln!("duragon: the system trash refused a note: {e:#}");
        }
        gone
    }

    /// Deletes a note and says where it went: `Duragon` when the system trash refused it. A note already deleted is
    /// not deleted again, but one still waiting for the system trash goes there.
    pub fn delete(&self, id: &NoteId, mode: DeleteMode) -> Result<DeleteMode> {
        let _lock = self.lock()?;
        let mut written = None;
        self.db.take_note(&id.to_string(), |bundle| {
            match mode {
                DeleteMode::Duragon | DeleteMode::System => written = Some(self.write(id, bundle)?),
                DeleteMode::Permanent => self.remove(id)?,
            }
            Ok(())
        })?;
        if mode != DeleteMode::System {
            return Ok(mode);
        }
        // The note is deleted, so nothing from here on is a failed delete.
        let paths = match written {
            Some(path) => vec![path],
            None => match self.copies(id) {
                Ok(copies) => copies.into_iter().map(|c| c.path).collect(),
                Err(_) => return Ok(DeleteMode::Duragon),
            },
        };
        let kept = paths.iter().filter(|p| !self.hand_over(p)).count();
        Ok(if kept == 0 { DeleteMode::System } else { DeleteMode::Duragon })
    }

    pub fn list(&self) -> Result<TrashList> {
        let _lock = self.lock()?;
        let live = self.live()?;
        let mut notes: Vec<TrashedNote> = Vec::new();
        let mut unreadable = 0;
        for path in self.files()? {
            match open(path) {
                Ok(c) if live.contains(&c.id.to_string()) => {}
                Ok(c) => notes.push(TrashedNote { id: c.id, title: c.file.bundle.note.meta.title, deleted_at: c.file.deleted_at }),
                Err(_) => unreadable += 1,
            }
        }
        notes.sort_by_key(|n| std::cmp::Reverse(n.deleted_at));
        let mut seen = HashSet::new();
        notes.retain(|n| seen.insert(n.id));
        for n in &mut notes {
            n.deleted_at = sane(n.deleted_at);
        }
        Ok(TrashList { notes, unreadable })
    }

    fn live(&self) -> Result<HashSet<String>> {
        Ok(self.db.list_notes()?.into_iter().map(|n| n.id).collect())
    }

    /// Puts a note back with everything it had. One that is already live is returned as it is.
    pub fn restore(&self, id: &NoteId) -> Result<NoteMeta> {
        let _lock = self.lock()?;
        if let Some(note) = self.db.find_note(&id.to_string())? {
            return Ok(note.meta);
        }
        let copy = self.copies(id)?.into_iter().next().ok_or_else(|| anyhow!("that note is not in the trash"))?;
        let meta = self.db.put_note(&cleaned(copy.file.bundle))?;
        // The note is back. A file that stays is hidden while it is.
        let _ = self.remove(id);
        Ok(meta)
    }

    /// Removes these notes from the trash, passing over any that is live.
    pub fn delete_forever(&self, ids: &[NoteId]) -> Result<()> {
        let _lock = self.lock()?;
        let live = self.live()?;
        for id in ids.iter().filter(|id| !live.contains(&id.to_string())) {
            self.remove(id)?;
        }
        Ok(())
    }
}
