//! One command per method of the frontend `Backend` interface (src/lib/api/tauri.ts).
use duragon_core::ai::local;
use duragon_core::consent::Asks;
use duragon_core::credentials;
use duragon_core::export::{export_notebook_markdown, from_markdown, to_docx, to_markdown};
use duragon_core::model::*;
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::AppState;

type Res<T> = Result<T, String>;

fn s<T>(r: anyhow::Result<T>) -> Res<T> {
    r.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn list_notes(state: State<'_, AppState>) -> Res<Vec<NoteMeta>> {
    let db = state.db.clone();
    blocking(move || db.list_notes()).await
}

#[tauri::command]
pub async fn search_notes(state: State<'_, AppState>, query: String) -> Res<Vec<NoteMeta>> {
    let db = state.db.clone();
    blocking(move || db.search_notes(&query)).await
}

#[tauri::command]
pub async fn get_note(state: State<'_, AppState>, id: String) -> Res<Note> {
    let db = state.db.clone();
    blocking(move || db.get_note(&id)).await
}

#[tauri::command]
pub async fn create_note(state: State<'_, AppState>, title: Option<String>) -> Res<Note> {
    let db = state.db.clone();
    blocking(move || db.create_note(title.as_deref())).await
}

#[tauri::command]
pub async fn save_note(
    state: State<'_, AppState>,
    id: String,
    title: String,
    doc: Box<serde_json::value::RawValue>,
    plain: String,
) -> Res<NoteMeta> {
    let db = state.db.clone();
    blocking(move || db.save_note_raw(&id, &title, doc.get(), &plain)).await
}

#[tauri::command]
pub async fn update_note_meta(state: State<'_, AppState>, id: String, patch: NoteMetaPatch) -> Res<Note> {
    s(state.db.update_note_meta(&id, &patch))
}

#[tauri::command]
pub async fn delete_note(state: State<'_, AppState>, id: NoteId, mode: DeleteMode) -> Res<DeleteMode> {
    let trash = state.trash.clone();
    blocking(move || trash.delete(&id, mode)).await
}

#[tauri::command]
pub async fn list_trash(state: State<'_, AppState>) -> Res<TrashList> {
    let trash = state.trash.clone();
    blocking(move || trash.list()).await
}

#[tauri::command]
pub async fn restore_note(state: State<'_, AppState>, id: NoteId) -> Res<NoteMeta> {
    let trash = state.trash.clone();
    blocking(move || trash.restore(&id)).await
}

#[tauri::command]
pub async fn delete_trashed(state: State<'_, AppState>, ids: Vec<NoteId>) -> Res<()> {
    let trash = state.trash.clone();
    blocking(move || trash.delete_forever(&ids)).await
}

#[tauri::command]
pub async fn list_folders(state: State<'_, AppState>) -> Res<Vec<Folder>> {
    s(state.db.list_folders())
}

#[tauri::command]
pub async fn create_folder(state: State<'_, AppState>, name: String) -> Res<Folder> {
    s(state.db.create_folder(&name))
}

#[tauri::command]
pub async fn rename_folder(state: State<'_, AppState>, id: String, name: String) -> Res<()> {
    s(state.db.rename_folder(&id, &name))
}

#[tauri::command]
pub async fn move_folder(state: State<'_, AppState>, id: String, parent_id: Option<String>) -> Res<()> {
    s(state.db.move_folder(&id, parent_id.as_deref()))
}

#[tauri::command]
pub async fn delete_folder(state: State<'_, AppState>, id: String) -> Res<()> {
    s(state.db.delete_folder(&id))
}

#[tauri::command]
pub async fn list_threads(state: State<'_, AppState>, note_id: String) -> Res<Vec<Thread>> {
    s(state.db.list_threads(&note_id))
}

#[tauri::command]
pub async fn set_thread_status(state: State<'_, AppState>, id: String, status: ThreadStatus) -> Res<()> {
    s(state.db.set_thread_status(&id, status))
}

#[tauri::command]
pub async fn set_thread_kind(state: State<'_, AppState>, id: String, kind: ThreadKind) -> Res<()> {
    s(state.db.set_thread_kind(&id, kind))
}

#[tauri::command]
pub async fn set_thread_pinned(state: State<'_, AppState>, id: String, pinned: bool) -> Res<()> {
    s(state.db.set_thread_pinned(&id, pinned))
}

#[tauri::command]
pub async fn thread_to_note(state: State<'_, AppState>, id: String) -> Res<Note> {
    s(state.db.thread_to_note(&id))
}

#[tauri::command]
pub async fn list_chat(state: State<'_, AppState>, note_id: String) -> Res<Vec<Message>> {
    s(state.db.list_chat(&note_id))
}

#[tauri::command]
pub async fn clear_chat(state: State<'_, AppState>, note_id: String) -> Res<()> {
    s(state.ai.clear_chat(&note_id))
}

#[tauri::command]
pub async fn get_chat_summary(state: State<'_, AppState>, note_id: String) -> Res<String> {
    s(state.db.get_chat_summary(&note_id).map(|c| c.text))
}

#[tauri::command]
pub async fn save_chat_summary(state: State<'_, AppState>, note_id: String, base: String, text: String) -> Res<bool> {
    s(state.db.edit_chat_summary(&note_id, &base, &text))
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Res<Settings> {
    s(state.db.get_settings())
}

#[tauri::command]
pub async fn save_settings(window: WebviewWindow, state: State<'_, AppState>, settings: Settings) -> Res<()> {
    let (db, credentials) = (state.db.clone(), state.credentials.clone());
    blocking(move || credentials::save_settings(&db, &credentials, &settings, |asks| allowed(&window, asks))).await
}

/// Asks in a window the page can't reach.
fn allowed(window: &WebviewWindow, asks: &Asks) -> bool {
    let mut message = String::new();
    if !asks.programs.is_empty() {
        message.push_str(&format!("Duragon will run this on your computer when a job uses it:\n\n{}\n\n", asks.programs.join("\n")));
    }
    if !asks.addresses.is_empty() {
        message.push_str(&format!("Duragon will send what it reads, and the provider’s key, here:\n\n{}\n\n", asks.addresses.join("\n\n")));
    }
    message.push_str("Allow it only if you just set it up yourself.");
    window
        .dialog()
        .message(message)
        .title(if asks.programs.is_empty() { "Use this address?" } else { "Run this command?" })
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom("Allow".into(), "Don’t allow".into()))
        .parent(window)
        .blocking_show()
}

#[tauri::command]
pub async fn list_credential_statuses(state: State<'_, AppState>) -> Res<Vec<ProviderCredentialStatus>> {
    let settings = s(state.db.get_settings())?;
    Ok(state.credentials.statuses(&settings.resolved_providers()))
}

#[tauri::command]
pub async fn set_provider_key(
    state: State<'_, AppState>,
    provider_id: String,
    key: String,
) -> Res<ProviderCredentialStatus> {
    s(credentials::set_provider_key(&state.db, &state.credentials, &provider_id, &key))
}

#[tauri::command]
pub async fn delete_provider_key(
    state: State<'_, AppState>,
    provider_id: String,
) -> Res<ProviderCredentialStatus> {
    s(credentials::delete_provider_key(&state.db, &state.credentials, &provider_id))
}

#[tauri::command]
pub async fn get_search_status(state: State<'_, AppState>) -> Res<SearchStatus> {
    s(credentials::search_status(&state.db, &state.credentials))
}

#[tauri::command]
pub async fn set_search_key(
    state: State<'_, AppState>,
    engine: SearchEngine,
    key: String,
) -> Res<SearchStatus> {
    s(credentials::set_search_key(&state.db, &state.credentials, engine, &key))
}

#[tauri::command]
pub async fn delete_search_key(
    state: State<'_, AppState>,
    engine: SearchEngine,
) -> Res<SearchStatus> {
    s(credentials::delete_search_key(&state.db, &state.credentials, engine))
}

#[tauri::command]
pub async fn local_models(state: State<'_, AppState>, provider_id: String) -> Res<Vec<String>> {
    s(local::local_models(&state.db, &state.credentials, &provider_id).await)
}

#[tauri::command]
pub async fn list_riddlers(state: State<'_, AppState>) -> Res<Vec<Riddler>> {
    s(state.db.list_riddlers())
}

#[tauri::command]
pub async fn save_riddler(state: State<'_, AppState>, riddler: Riddler) -> Res<Riddler> {
    s(state.db.save_riddler(&riddler))
}

#[tauri::command]
pub async fn delete_riddler(state: State<'_, AppState>, id: String) -> Res<()> {
    s(state.db.delete_riddler(&id))
}

fn log_failure(what: &str, r: anyhow::Result<()>) {
    if let Err(e) = r {
        eprintln!("duragon: {what} failed: {e:#}");
    }
}

#[tauri::command]
pub async fn ai_read(state: State<'_, AppState>, note_id: String, target: String, nth: usize) -> Res<()> {
    let ai = state.ai.clone();
    tauri::async_runtime::spawn(async move { log_failure("read", ai.read(&note_id, &target, nth).await) });
    Ok(())
}

#[tauri::command]
pub async fn ai_answer(state: State<'_, AppState>, note_id: String, target: String, nth: usize) -> Res<()> {
    let ai = state.ai.clone();
    tauri::async_runtime::spawn(async move { log_failure("answer", ai.answer(&note_id, &target, nth).await) });
    Ok(())
}

#[tauri::command]
pub async fn ai_reply(state: State<'_, AppState>, thread_id: String, text: String) -> Res<()> {
    s(state.db.get_thread(&thread_id))?;
    let ai = state.ai.clone();
    tauri::async_runtime::spawn(async move { log_failure("reply", ai.reply(&thread_id, &text).await) });
    Ok(())
}

#[tauri::command]
pub async fn ai_chat(state: State<'_, AppState>, note_id: String, text: String, near: String) -> Res<()> {
    s(state.db.get_note(&note_id))?;
    let ai = state.ai.clone();
    tauri::async_runtime::spawn(async move { log_failure("chat", ai.chat(&note_id, &text, &near).await) });
    Ok(())
}

#[tauri::command]
pub async fn ai_ask(state: State<'_, AppState>, note_id: String, target: String, question: Option<String>, nth: usize) -> Res<()> {
    let ai = state.ai.clone();
    tauri::async_runtime::spawn(async move {
        log_failure("ask", ai.ask_selection(&note_id, &target, question.as_deref(), nth).await)
    });
    Ok(())
}

#[tauri::command]
pub async fn ai_visualize(state: State<'_, AppState>, note_id: String, target: String) -> Res<()> {
    let ai = state.ai.clone();
    tauri::async_runtime::spawn(async move { log_failure("visualize", ai.visualize(&note_id, &target).await) });
    Ok(())
}

#[tauri::command]
pub fn ai_cancel_read(note_id: String) {
    duragon_core::ai::cancel(&note_id, AiTask::Read);
}

#[tauri::command]
pub async fn export_note(state: State<'_, AppState>, id: String, format: ExportFormat, path: String) -> Res<()> {
    let db = state.db.clone();
    blocking(move || {
        let note = db.get_note(&id)?;
        match format {
            ExportFormat::Md => std::fs::write(&path, to_markdown(&note.doc))?,
            ExportFormat::Docx => to_docx(&note.doc, std::path::Path::new(&path))?,
        }
        Ok(())
    })
    .await
}

const MAX_IMPORT_BYTES: u64 = 5 * 1024 * 1024;

#[tauri::command]
pub async fn import_markdown(state: State<'_, AppState>, path: String, folder_id: Option<String>) -> Res<Note> {
    let db = state.db.clone();
    blocking(move || {
        use std::io::Read;
        let file_path = std::path::Path::new(&path);
        let meta = std::fs::metadata(file_path)?;
        if meta.len() > MAX_IMPORT_BYTES {
            anyhow::bail!("Markdown file is larger than 5 MB");
        }
        let file = std::fs::File::open(file_path)?;
        let mut bytes = Vec::with_capacity(meta.len() as usize);
        file.take(MAX_IMPORT_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_IMPORT_BYTES {
            anyhow::bail!("Markdown file is larger than 5 MB");
        }
        let md = String::from_utf8(bytes).map_err(|_| anyhow::anyhow!("file is not valid UTF-8 text"))?;
        let fallback = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");
        let (title, document) = from_markdown(&md, fallback);
        let note = db.create_note_with_doc(&title, document)?;
        if let Some(fid) = folder_id {
            db.update_note_meta(&note.meta.id, &NoteMetaPatch { folder_id: Some(Some(fid)), ..Default::default() })
        } else {
            Ok(note)
        }
    })
    .await
}

#[tauri::command]
pub async fn export_markdown_dir(state: State<'_, AppState>, dir: String, folder_id: Option<String>) -> Res<usize> {
    let db = state.db.clone();
    blocking(move || export_notebook_markdown(&db, std::path::Path::new(&dir), folder_id.as_deref())).await
}

#[tauri::command]
pub async fn voice_status(state: State<'_, AppState>) -> Res<crate::voice_client::VoiceStatus> {
    Ok(state.voice.status())
}

#[tauri::command]
pub async fn voice_install(state: State<'_, AppState>, kind: crate::voice_client::ModelKind) -> Res<()> {
    s(state.voice.install(kind).await)
}

#[tauri::command]
pub async fn list_voices(state: State<'_, AppState>, refresh: bool) -> Res<Vec<crate::voice_client::VoiceOption>> {
    let voice = state.voice.clone();
    blocking(move || Ok(voice.voices(refresh))).await
}

#[tauri::command]
pub async fn list_listen_models(state: State<'_, AppState>) -> Res<Vec<crate::voice_client::VoiceOption>> {
    let voice = state.voice.clone();
    blocking(move || Ok(voice.listen_models().iter().map(|m| m.option()).collect())).await
}

/// Blocking SQLite, export, and trash I/O runs off the async runtime.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> Res<T> {
    s(tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn listen_start(state: State<'_, AppState>, hands_free: bool) -> Res<()> {
    let settings = s(state.db.get_settings())?;
    s(state.voice.listen_start(hands_free, &settings.listen_model).await)
}

#[tauri::command]
pub async fn listen_stop(state: State<'_, AppState>) -> Res<String> {
    s(state.voice.listen_stop().await)
}

/// Speaks with the chosen voice, or with `voice` when one is given, as for a sample.
#[tauri::command]
pub async fn speak(state: State<'_, AppState>, message_id: Option<String>, text: String, voice: Option<String>) -> Res<()> {
    let settings = s(state.db.get_settings())?;
    let voice = voice.unwrap_or(settings.voice_name);
    s(state.voice.speak(message_id, &text, &voice, settings.speaking_speed as f32).await)
}

#[tauri::command]
pub async fn stop_speaking(state: State<'_, AppState>) -> Res<()> {
    s(state.voice.stop_speaking().await)
}

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        list_notes, search_notes, get_note, create_note, save_note, update_note_meta, delete_note,
        list_trash, restore_note, delete_trashed,
        list_folders, create_folder, rename_folder, move_folder, delete_folder,
        list_threads, set_thread_status, set_thread_pinned, set_thread_kind, thread_to_note, list_chat,
        get_chat_summary, save_chat_summary, clear_chat,
        get_settings, save_settings, list_credential_statuses, set_provider_key, delete_provider_key,
        get_search_status, set_search_key, delete_search_key, local_models,
        list_riddlers, save_riddler, delete_riddler,
        ai_read, ai_answer, ai_reply, ai_chat, ai_ask, ai_visualize, ai_cancel_read, export_note, import_markdown, export_markdown_dir,
        voice_status, voice_install, list_voices, list_listen_models, listen_start, listen_stop, speak, stop_speaking,
    ]
}
