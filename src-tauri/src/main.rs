#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod system_trash;
mod voice_client;

use duragon_core::ai::notes_mcp::{self, NotesSearch};
use duragon_core::ai::{Ai, Events};
use duragon_core::credentials::CredentialsStore;
use duragon_core::db::Db;
use duragon_core::model::Appearance;
use duragon_core::trash::Trash;
use std::sync::Arc;
use tauri::webview::PageLoadEvent;
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, Url, Webview, WebviewWindow, WebviewWindowBuilder};
use voice_client::VoiceClient;

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

/// Whether the window may go there. Sources and anything else on the web open in the browser instead.
fn own_page(url: &Url, dev: Option<&Url>) -> bool {
    match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        // How Windows serves the same page.
        "http" | "https" if url.host_str() == Some("tauri.localhost") => cfg!(windows),
        _ => dev.is_some_and(|dev| dev.origin().is_tuple() && dev.origin() == url.origin()),
    }
}

/// Whether the page will find the system dark. WebKit asks GTK, which follows the portal when there is one.
#[cfg(target_os = "linux")]
fn system_is_dark(_: &WebviewWindow) -> tauri::Result<bool> {
    use gtk::prelude::GtkSettingsExt;
    Ok(gtk::Settings::default().is_some_and(|s| {
        s.is_gtk_application_prefer_dark_theme() || s.gtk_theme_name().is_some_and(|name| name.to_lowercase().ends_with("-dark"))
    }))
}

#[cfg(not(target_os = "linux"))]
fn system_is_dark(window: &WebviewWindow) -> tauri::Result<bool> {
    Ok(window.theme()? == tauri::Theme::Dark)
}

/// The window opens in the page's colour and the page is shown once loaded: WebKitGTK's own first frame can be black.
const PAGE_WAITS: bool = cfg!(target_os = "linux");

fn show_page(page: &Webview) -> tauri::Result<()> {
    page.show()?;
    // Hiding took it away.
    page.set_focus()
}

fn open_window(app: &tauri::App, appearance: Appearance) -> tauri::Result<()> {
    let config = &app.config().app.windows[0];
    // As settings.svelte.ts will choose, but before the first paint.
    let theme = match appearance {
        Appearance::System => "matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'".to_string(),
        chosen => format!("'{}'", chosen.as_str()),
    };
    let dev = app.config().build.dev_url.clone().filter(|_| tauri::is_dev());
    let window = WebviewWindowBuilder::from_config(app.handle(), config)?
        .initialization_script(format!("document.documentElement.dataset.theme = {theme};"))
        .on_navigation(move |url| own_page(url, dev.as_ref()))
        .on_page_load(|window, page| {
            if PAGE_WAITS && page.event() == PageLoadEvent::Finished {
                if let Err(e) = show_page(window.as_ref()) {
                    eprintln!("duragon: could not show the page: {e}");
                }
            }
        })
        .build()?;
    let dark = match appearance {
        Appearance::Dark => true,
        Appearance::Light => false,
        Appearance::System => system_is_dark(&window)?,
    };
    window.set_background_color(Some(paper(dark)))?;
    if PAGE_WAITS {
        AsRef::<Webview>::as_ref(&window).hide()?;
    }
    Ok(())
}

pub struct AppState {
    pub db: Db,
    pub trash: Trash,
    pub ai: Ai,
    pub voice: VoiceClient,
    pub credentials: CredentialsStore,
}

/// `duragon --notes-mcp <db> <open note id>`: the side chat's notes search server, spawned by the `claude` CLI.
fn serve_notes_mcp(db: &str, exclude: &str) -> anyhow::Result<()> {
    let db = Db::open_read_only(std::path::Path::new(db))?;
    notes_mcp::serve(&db, exclude, std::io::stdin().lock(), std::io::stdout().lock())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let [_, flag, db, exclude] = args.as_slice() {
        if flag == "--notes-mcp" {
            if let Err(e) = serve_notes_mcp(db, exclude) {
                eprintln!("duragon: notes search failed: {e}");
                std::process::exit(1);
            }
            return;
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let db_path = dir.join("duragon.db");
            let db = Db::open(&db_path)?;
            open_window(app, db.get_settings().map(|s| s.appearance).unwrap_or(Appearance::System))?;
            let credentials = CredentialsStore::new(dir.join("credentials.json"));
            let mut ai = Ai::new(db.clone(), Arc::new(TauriEvents(app.handle().clone())));
            ai.credentials = Some(credentials.clone());
            match std::env::current_exe() {
                Ok(exe) => ai.notes_search = Some(NotesSearch { exe, db: db_path }),
                Err(e) => eprintln!("duragon: notes search unavailable: {e}"),
            }
            let handle = app.handle().clone();
            let emit: voice_client::Emit = Arc::new(move |name, payload| {
                if let Err(e) = handle.emit(name, payload) {
                    eprintln!("duragon: could not emit {name}: {e}");
                }
            });
            let voice = VoiceClient::new(dir.join("models"), emit);
            let trash = Trash::new(db.clone(), dir.join(".trash"), Arc::new(system_trash::OsTrash));
            app.manage(AppState { db, trash, ai, voice, credentials });
            Ok(())
        })
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())
        .expect("error while running Duragon");
}

#[cfg(test)]
mod tests {
    use super::own_page;
    use tauri::Url;

    fn goes(to: &str, dev: Option<&str>) -> bool {
        own_page(&Url::parse(to).unwrap(), dev.map(|d| Url::parse(d).unwrap()).as_ref())
    }

    #[test]
    fn the_window_stays_on_the_apps_pages() {
        assert!(goes("tauri://localhost/", None));
        assert!(goes("tauri://localhost/index.html#x", None));
        assert_eq!(goes("http://tauri.localhost/", None), cfg!(windows));
        for away in [
            "https://example.com/", "http://localhost:1420/", "http://127.0.0.1/", "file:///etc/hosts", "data:text/html,x",
            "about:blank", "javascript:0", "tauri://evil/", "https://tauri.localhost.evil.com/", "http://tauri.localhost@evil.com/",
        ] {
            assert!(!goes(away, None), "{away}");
        }
    }

    #[test]
    fn dev_adds_only_the_dev_server() {
        let dev = Some("http://localhost:1420");
        assert!(goes("http://localhost:1420/src/x", dev));
        for away in ["http://localhost:1421/", "https://localhost:1420/", "http://localhost.evil.com:1420/", "http://localhost:1420@evil.com/", "data:text/html,x"] {
            assert!(!goes(away, dev), "{away}");
        }
    }
}
