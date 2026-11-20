#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use duragon_core::ai::{Ai, Events};
use duragon_core::db::Db;
use duragon_voice::Voice;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

struct TauriEvents(AppHandle);

impl Events for TauriEvents {
    fn emit(&self, name: &str, payload: serde_json::Value) {
        if let Err(e) = self.0.emit(name, payload) {
            eprintln!("duragon: could not emit {name}: {e}");
        }
    }
}

pub struct AppState {
    pub db: Db,
    pub ai: Ai,
    pub voice: Voice,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let db = Db::open(&dir.join("duragon.db"))?;
            let ai = Ai::new(db.clone(), Arc::new(TauriEvents(app.handle().clone())));
            let handle = app.handle().clone();
            let emit: duragon_voice::Emit = Arc::new(move |name, payload| {
                if let Err(e) = handle.emit(name, payload) {
                    eprintln!("duragon: could not emit {name}: {e}");
                }
            });
            let voice = Voice::new(dir.join("models"), emit);
            app.manage(AppState { db, ai, voice });
            Ok(())
        })
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())
        .expect("error while running Duragon");
}
