//! One command per method of the frontend `Backend` interface (src/lib/api/tauri.ts).
use duragon_core::ai::local;
use duragon_core::consent::Asks;
use duragon_core::credentials;
use duragon_core::db::Db;
use duragon_core::export::{export_notebook_markdown, import_markdown_file, to_docx, to_markdown};
use duragon_core::model::*;
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::AppState;

type Res<T> = Result<T, String>;

fn s<T>(r: anyhow::Result<T>) -> Res<T> {
    r.map_err(|e| format!("{e:#}"))
}

async fn db_blocking<T: Send + 'static>(
    state: &State<'_, AppState>,
    f: impl FnOnce(Db) -> anyhow::Result<T> + Send + 'static,
) -> Res<T> {
    let db = state.db.clone();
    blocking(move || f(db)).await
}

#[tauri::command]
pub async fn list_notes(state: State<'_, AppState>) -> Res<Vec<NoteMeta>> {
    db_blocking(&state, |db| db.list_notes()).await
}

#[tauri::command]
pub async fn search_notes(state: State<'_, AppState>, query: String) -> Res<Vec<NoteMeta>> {
    db_blocking(&state, move |db| db.search_notes(&query)).await
}

#[tauri::command]
pub async fn list_backlinks(state: State<'_, AppState>, note_id: String) -> Res<Vec<NoteMeta>> {
    db_blocking(&state, move |db| db.list_backlinks(&note_id)).await
}

#[tauri::command]
pub async fn get_note(state: State<'_, AppState>, id: String) -> Res<Note> {
    db_blocking(&state, move |db| db.get_note(&id)).await
}

#[tauri::command]
pub async fn create_note(state: State<'_, AppState>, title: Option<String>) -> Res<Note> {
    db_blocking(&state, move |db| db.create_note(title.as_deref())).await
}

#[tauri::command]
pub async fn save_note(
    state: State<'_, AppState>,
    id: String,
    title: String,
    doc: Box<serde_json::value::RawValue>,
    plain: String,
) -> Res<NoteMeta> {
    db_blocking(&state, move |db| db.save_note_raw(&id, &title, doc.get(), &plain)).await
}

#[tauri::command]
pub async fn list_revisions(state: State<'_, AppState>, note_id: String) -> Res<Vec<NoteRevision>> {
    db_blocking(&state, move |db| db.list_revisions(&note_id)).await
}

#[tauri::command]
pub async fn restore_revision(state: State<'_, AppState>, note_id: String, revision_id: String) -> Res<Note> {
    db_blocking(&state, move |db| db.restore_revision(&note_id, &revision_id)).await
}

#[tauri::command]
pub async fn update_note_meta(state: State<'_, AppState>, id: String, patch: NoteMetaPatch) -> Res<Note> {
    db_blocking(&state, move |db| db.update_note_meta(&id, &patch)).await
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
    db_blocking(&state, |db| db.list_folders()).await
}

#[tauri::command]
pub async fn create_folder(state: State<'_, AppState>, name: String) -> Res<Folder> {
    db_blocking(&state, move |db| db.create_folder(&name)).await
}

#[tauri::command]
pub async fn rename_folder(state: State<'_, AppState>, id: String, name: String) -> Res<()> {
    db_blocking(&state, move |db| db.rename_folder(&id, &name)).await
}

#[tauri::command]
pub async fn move_folder(state: State<'_, AppState>, id: String, parent_id: Option<String>) -> Res<()> {
    db_blocking(&state, move |db| db.move_folder(&id, parent_id.as_deref())).await
}

#[tauri::command]
pub async fn delete_folder(state: State<'_, AppState>, id: String) -> Res<()> {
    db_blocking(&state, move |db| db.delete_folder(&id)).await
}

#[tauri::command]
pub async fn list_threads(state: State<'_, AppState>, note_id: String) -> Res<Vec<Thread>> {
    db_blocking(&state, move |db| db.list_threads(&note_id)).await
}

#[tauri::command]
pub async fn set_thread_status(state: State<'_, AppState>, id: String, status: ThreadStatus) -> Res<()> {
    db_blocking(&state, move |db| db.set_thread_status(&id, status)).await
}

#[tauri::command]
pub async fn set_thread_kind(state: State<'_, AppState>, id: String, kind: ThreadKind) -> Res<()> {
    db_blocking(&state, move |db| db.set_thread_kind(&id, kind)).await
}

#[tauri::command]
pub async fn set_thread_pinned(state: State<'_, AppState>, id: String, pinned: bool) -> Res<()> {
    db_blocking(&state, move |db| db.set_thread_pinned(&id, pinned)).await
}

#[tauri::command]
pub async fn thread_to_note(state: State<'_, AppState>, id: String) -> Res<Note> {
    db_blocking(&state, move |db| db.thread_to_note(&id)).await
}

#[tauri::command]
pub async fn list_chat(state: State<'_, AppState>, note_id: String) -> Res<Vec<Message>> {
    db_blocking(&state, move |db| db.list_chat(&note_id)).await
}

#[tauri::command]
pub async fn clear_chat(state: State<'_, AppState>, note_id: String) -> Res<()> {
    let ai = state.ai.clone();
    blocking(move || ai.clear_chat(&note_id)).await
}

#[tauri::command]
pub async fn get_chat_summary(state: State<'_, AppState>, note_id: String) -> Res<String> {
    db_blocking(&state, move |db| db.get_chat_summary(&note_id).map(|c| c.text)).await
}

#[tauri::command]
pub async fn save_chat_summary(state: State<'_, AppState>, note_id: String, base: String, text: String) -> Res<bool> {
    db_blocking(&state, move |db| db.edit_chat_summary(&note_id, &base, &text)).await
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Res<Settings> {
    db_blocking(&state, |db| db.get_settings()).await
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
    let (db, credentials) = (state.db.clone(), state.credentials.clone());
    blocking(move || {
        let settings = db.get_settings()?;
        Ok(credentials.statuses(&settings.resolved_providers()))
    })
    .await
}

#[tauri::command]
pub async fn set_provider_key(
    state: State<'_, AppState>,
    provider_id: String,
    key: String,
) -> Res<ProviderCredentialStatus> {
    let (db, credentials) = (state.db.clone(), state.credentials.clone());
    blocking(move || credentials::set_provider_key(&db, &credentials, &provider_id, &key)).await
}

#[tauri::command]
pub async fn delete_provider_key(
    state: State<'_, AppState>,
    provider_id: String,
) -> Res<ProviderCredentialStatus> {
    let (db, credentials) = (state.db.clone(), state.credentials.clone());
    blocking(move || credentials::delete_provider_key(&db, &credentials, &provider_id)).await
}

#[tauri::command]
pub async fn get_search_status(state: State<'_, AppState>) -> Res<SearchStatus> {
    let (db, credentials) = (state.db.clone(), state.credentials.clone());
    blocking(move || credentials::search_status(&db, &credentials)).await
}

#[tauri::command]
pub async fn set_search_key(
    state: State<'_, AppState>,
    engine: SearchEngine,
    key: String,
) -> Res<SearchStatus> {
    let (db, credentials) = (state.db.clone(), state.credentials.clone());
    blocking(move || credentials::set_search_key(&db, &credentials, engine, &key)).await
}

#[tauri::command]
pub async fn delete_search_key(
    state: State<'_, AppState>,
    engine: SearchEngine,
) -> Res<SearchStatus> {
    let (db, credentials) = (state.db.clone(), state.credentials.clone());
    blocking(move || credentials::delete_search_key(&db, &credentials, engine)).await
}

#[tauri::command]
pub async fn local_models(state: State<'_, AppState>, provider_id: String) -> Res<Vec<String>> {
    s(local::local_models(&state.db, &state.credentials, &provider_id).await)
}

#[tauri::command]
pub async fn list_riddlers(state: State<'_, AppState>) -> Res<Vec<Riddler>> {
    db_blocking(&state, |db| db.list_riddlers()).await
}

#[tauri::command]
pub async fn save_riddler(state: State<'_, AppState>, riddler: Riddler) -> Res<Riddler> {
    db_blocking(&state, move |db| db.save_riddler(&riddler)).await
}

#[tauri::command]
pub async fn delete_riddler(state: State<'_, AppState>, id: String) -> Res<()> {
    db_blocking(&state, move |db| db.delete_riddler(&id)).await
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
    let tid = thread_id.clone();
    db_blocking(&state, move |db| db.get_thread(&tid)).await?;
    let ai = state.ai.clone();
    tauri::async_runtime::spawn(async move { log_failure("reply", ai.reply(&thread_id, &text).await) });
    Ok(())
}

#[tauri::command]
pub async fn ai_chat(state: State<'_, AppState>, note_id: String, text: String, near: String) -> Res<()> {
    let nid = note_id.clone();
    db_blocking(&state, move |db| db.get_note(&nid)).await?;
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
    db_blocking(&state, move |db| {
        let note = db.get_note(&id)?;
        match format {
            ExportFormat::Md => std::fs::write(&path, to_markdown(&note.doc))?,
            ExportFormat::Docx => to_docx(&note.doc, std::path::Path::new(&path))?,
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn import_markdown(state: State<'_, AppState>, path: String, folder_id: Option<String>) -> Res<Note> {
    db_blocking(&state, move |db| import_markdown_file(&db, std::path::Path::new(&path), folder_id.as_deref())).await
}

#[tauri::command]
pub async fn export_markdown_dir(state: State<'_, AppState>, dir: String, folder_id: Option<String>) -> Res<usize> {
    db_blocking(&state, move |db| export_notebook_markdown(&db, std::path::Path::new(&dir), folder_id.as_deref())).await
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
    let settings = db_blocking(&state, |db| db.get_settings()).await?;
    s(state.voice.listen_start(hands_free, &settings.listen_model).await)
}

#[tauri::command]
pub async fn listen_stop(state: State<'_, AppState>) -> Res<String> {
    s(state.voice.listen_stop().await)
}

/// Speaks with the chosen voice, or with `voice` when one is given, as for a sample.
#[tauri::command]
pub async fn speak(state: State<'_, AppState>, message_id: Option<String>, text: String, voice: Option<String>) -> Res<()> {
    let settings = db_blocking(&state, |db| db.get_settings()).await?;
    let voice = voice.unwrap_or(settings.voice_name);
    s(state.voice.speak(message_id, &text, &voice, settings.speaking_speed as f32).await)
}

#[tauri::command]
pub async fn stop_speaking(state: State<'_, AppState>) -> Res<()> {
    s(state.voice.stop_speaking().await)
}

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        list_notes, search_notes, list_backlinks, get_note, create_note, save_note, update_note_meta, delete_note,
        list_revisions, restore_revision,
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
