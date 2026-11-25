//! One command per method of the frontend `Backend` interface (src/lib/api/tauri.ts).
use duragon_core::ai::local;
use duragon_core::credentials;
use duragon_core::export::{to_docx, to_markdown};
use duragon_core::model::*;
use tauri::State;

use crate::AppState;

type Res<T> = Result<T, String>;

fn s<T>(r: anyhow::Result<T>) -> Res<T> {
    r.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn list_notes(state: State<'_, AppState>) -> Res<Vec<NoteMeta>> {
    s(state.db.list_notes())
}

#[tauri::command]
pub async fn search_notes(state: State<'_, AppState>, query: String) -> Res<Vec<NoteMeta>> {
    s(state.db.search_notes(&query))
}

#[tauri::command]
pub async fn get_note(state: State<'_, AppState>, id: String) -> Res<Note> {
    s(state.db.get_note(&id))
}

#[tauri::command]
pub async fn create_note(state: State<'_, AppState>, title: Option<String>) -> Res<Note> {
    s(state.db.create_note(title.as_deref()))
}

#[tauri::command]
pub async fn save_note(
    state: State<'_, AppState>,
    id: String,
    title: String,
    doc: serde_json::Value,
    plain: String,
) -> Res<NoteMeta> {
    s(state.db.save_note(&id, &title, &doc, &plain))
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
pub async fn save_settings(state: State<'_, AppState>, settings: Settings) -> Res<()> {
    s(credentials::save_settings(&state.db, &state.credentials, &settings))
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
    let note = s(state.db.get_note(&id))?;
    match format {
        ExportFormat::Md => std::fs::write(&path, to_markdown(&note.doc)).map_err(|e| e.to_string()),
        ExportFormat::Docx => s(to_docx(&note.doc, std::path::Path::new(&path))),
    }
}

#[tauri::command]
pub async fn voice_status(state: State<'_, AppState>) -> Res<duragon_voice::VoiceStatus> {
    Ok(state.voice.status())
}

#[tauri::command]
pub async fn voice_install(state: State<'_, AppState>) -> Res<()> {
    s(state.voice.install().await)
}

#[tauri::command]
pub async fn list_voices(state: State<'_, AppState>) -> Res<Vec<duragon_voice::VoiceOption>> {
    Ok(state.voice.voices())
}

/// Model loading, transcription and the trash's files block, so they run off the async runtime.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> Res<T> {
    s(tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn listen_start(state: State<'_, AppState>, hands_free: bool) -> Res<()> {
    let voice = state.voice.clone();
    blocking(move || voice.listen_start(hands_free)).await
}

#[tauri::command]
pub async fn listen_stop(state: State<'_, AppState>) -> Res<String> {
    let voice = state.voice.clone();
    blocking(move || voice.listen_stop()).await
}

#[tauri::command]
pub async fn speak(state: State<'_, AppState>, message_id: Option<String>, text: String) -> Res<()> {
    let settings = s(state.db.get_settings())?;
    let voice = state.voice.clone();
    blocking(move || voice.speak(message_id, &text, &settings.voice_name, settings.speaking_speed as f32)).await
}

#[tauri::command]
pub async fn stop_speaking(state: State<'_, AppState>) -> Res<()> {
    let voice = state.voice.clone();
    blocking(move || {
        voice.stop_speaking();
        Ok(())
    })
    .await
}

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        list_notes, search_notes, get_note, create_note, save_note, update_note_meta, delete_note,
        list_trash, restore_note, delete_trashed,
        list_folders, create_folder, rename_folder, move_folder, delete_folder,
        list_threads, set_thread_status, set_thread_pinned, set_thread_kind, thread_to_note, list_chat,
        get_chat_summary, save_chat_summary, clear_chat,
        get_settings, save_settings, list_credential_statuses, set_provider_key, delete_provider_key, local_models,
        list_riddlers, save_riddler, delete_riddler,
        ai_read, ai_answer, ai_reply, ai_chat, ai_ask, ai_visualize, ai_cancel_read, export_note,
        voice_status, voice_install, list_voices, listen_start, listen_stop, speak, stop_speaking,
    ]
}
