#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use duragon_core::ai::{Ai, Events};
use duragon_core::db::Db;
use duragon_core::model::Appearance;
use duragon_voice::Voice;
use std::sync::Arc;
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, Theme, WebviewWindowBuilder};

struct TauriEvents(AppHandle);

impl Events for TauriEvents {
    fn emit(&self, name: &str, payload: serde_json::Value) {
        if let Err(e) = self.0.emit(name, payload) {
            eprintln!("duragon: could not emit {name}: {e}");
        }
    }
}

/// The page's --paper in each theme (src/styles/tokens.css), painted by the window before the page can.
fn paper(dark: bool) -> Color {
    if dark { Color(0x1a, 0x1a, 0x19, 0xff) } else { Color(0xf8, 0xf7, 0xf4, 0xff) }
}

/// Opens the main window in the note's colours, so nothing white shows while the page loads.
fn open_window(app: &tauri::App, appearance: Appearance) -> tauri::Result<()> {
    let config = &app.config().app.windows[0];
    let mut builder = WebviewWindowBuilder::from_config(app.handle(), config)?;
    if appearance != Appearance::System {
        builder = builder.initialization_script(format!("document.documentElement.dataset.theme = '{}';", appearance.as_str()));
    }
    let window = builder.build()?;
    let dark = match appearance {
        Appearance::Dark => true,
        Appearance::Light => false,
        Appearance::System => window.theme()? == Theme::Dark,
    };
    window.set_background_color(Some(paper(dark)))?;
    Ok(())
}

pub struct AppState {
    pub db: Db,
    pub ai: Ai,
    pub voice: Voice,
}

fn main() {
    // WebKitGTK on native Wayland paints at half the display rate (30 fps on a 60 Hz screen), which makes every
    // transition stutter. Through XWayland it keeps up. Falls back to Wayland where there is no X server.
    #[cfg(target_os = "linux")]
    if std::env::var_os("GDK_BACKEND").is_none() {
        std::env::set_var("GDK_BACKEND", "x11,wayland");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let db = Db::open(&dir.join("duragon.db"))?;
            open_window(app, db.get_settings().map(|s| s.appearance).unwrap_or(Appearance::System))?;
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
