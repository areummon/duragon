mod common;
use common::{fake_claude, Capture};
use duragon_core::ai::Ai;
use duragon_core::credentials::{delete_provider_key, save_settings, set_provider_key, CredentialsStore};
use duragon_core::db::Db;
use duragon_core::model::{ProviderConfig, ProviderKind, Settings};
use serde_json::{json, Value};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

struct App {
    db: Db,
    store: CredentialsStore,
    file: PathBuf,
    dir: TempDir,
}

fn app() -> App {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("credentials.json");
    App {
        db: Db::open(&dir.path().join("duragon.db")).unwrap(),
        store: CredentialsStore::new(file.clone()),
        file,
        dir,
    }
}

fn custom(id: &str) -> ProviderConfig {
    ProviderConfig {
        id: id.into(),
        name: "Mine".into(),
        kind: ProviderKind::OpenAi,
        base_url: Some("http://127.0.0.1:9/v1".into()),
        command: None,
        args: vec![],
        env_var: None,
        models: vec![],
        builtin: false,
    }
}

fn settings_with(ids: &[&str]) -> Settings {
    let mut settings = Settings::default();
    settings.providers.extend(ids.iter().map(|id| custom(id)));
    settings
}

impl App {
    fn save(&self, settings: &Settings) -> anyhow::Result<()> {
        save_settings(&self.db, &self.store, settings)
    }

    fn set(&self, id: &str, key: &str) -> anyhow::Result<String> {
        set_provider_key(&self.db, &self.store, id, key).map(|s| s.provider_id)
    }

    fn has_key(&self, id: &str) -> bool {
        self.store.status_for(&custom(id)).has_stored_key
    }
}

#[test]
fn a_removed_provider_takes_its_key_with_it() {
    let app = app();
    app.save(&settings_with(&["mine", "kept"])).unwrap();
    app.set("mine", "sk-mine-1234567890").unwrap();
    app.set("kept", "sk-kept-1234567890").unwrap();
    app.set("openai", "sk-openai-1234567890").unwrap();

    app.save(&settings_with(&["kept"])).unwrap();
    assert!(!std::fs::read_to_string(&app.file).unwrap().contains("sk-mine"), "the key is still on disk");

    app.save(&settings_with(&["mine", "kept"])).unwrap();
    assert!(!app.has_key("mine"), "a new provider inherited the old key");
    assert!(app.has_key("kept"));
    assert!(app.has_key("openai"));
}

#[test]
fn a_provider_whose_key_cannot_be_dropped_is_not_removed() {
    let app = app();
    app.save(&settings_with(&["mine"])).unwrap();
    std::fs::write(&app.file, DAMAGED[3]).unwrap();
    let before = app.db.get_settings().unwrap();

    let mut without = settings_with(&[]);
    without.just_my_words = true;
    assert!(app.save(&without).is_err());
    assert_eq!(app.db.get_settings().unwrap(), before, "the provider is gone and its key is not");
    assert_eq!(std::fs::read(&app.file).unwrap(), DAMAGED[3]);
}

#[test]
fn settings_that_cannot_be_read_remove_no_provider() {
    for row in ["not json", r#"{"providers":[{"id":"mine","kind":"from-a-newer-duragon"}]}"#] {
        let app = app();
        app.save(&settings_with(&["mine"])).unwrap();
        app.set("mine", "sk-mine-1234567890").unwrap();
        let db = rusqlite::Connection::open(app.dir.path().join("duragon.db")).unwrap();
        assert_eq!(db.execute("UPDATE kv SET value=?1 WHERE key='settings'", [row]).unwrap(), 1);

        app.save(&app.db.get_settings().unwrap()).unwrap();
        assert!(app.has_key("mine"), "reading {row} as defaults dropped a key");
    }
}

#[test]
fn a_built_in_provider_missing_from_the_saved_list_keeps_its_key() {
    let app = app();
    app.set("openai", "sk-openai-1234567890").unwrap();
    app.save(&Settings { providers: vec![custom("mine")], ..Settings::default() }).unwrap();
    assert!(app.has_key("openai"));
}

#[test]
fn saving_settings_leaves_a_file_with_nothing_to_drop_alone() {
    let app = app();
    app.save(&settings_with(&["mine"])).unwrap();
    assert!(!app.file.exists());

    app.set("mine", "sk-mine-1234567890").unwrap();
    let before = std::fs::metadata(&app.file).unwrap();
    app.save(&settings_with(&["mine"])).unwrap();
    let after = std::fs::metadata(&app.file).unwrap();
    assert_eq!((before.ino(), before.mtime_nsec()), (after.ino(), after.mtime_nsec()));
}

#[test]
fn a_key_for_an_unknown_provider_is_refused() {
    let app = app();
    app.save(&settings_with(&["mine"])).unwrap();
    app.set("mine", "sk-mine-1234567890").unwrap();

    let set = app.set("gone", "sk-gone-1234567890");
    assert!(set.is_err(), "answered for {set:?}");
    assert!(!app.has_key("gone"));
    let deleted = delete_provider_key(&app.db, &app.store, "gone").map(|s| s.provider_id);
    assert!(deleted.is_err(), "answered for {deleted:?}");

    assert_eq!(app.set("mine", "sk-mine-0987654321").unwrap(), "mine");
    assert_eq!(delete_provider_key(&app.db, &app.store, "mine").unwrap().provider_id, "mine");
    assert!(!app.has_key("mine"));
}

/// Each holds a key the app can no longer read, or a shape it doesn't know.
const DAMAGED: [&[u8]; 8] = [
    b"",
    b"null",
    b"{}",
    br#"{"keys":{"openai":"sk-secret-1234"#,
    br#"{"keys":{"openai":["sk-secret-1234"]}}"#,
    br#""sk-secret-1234""#,
    br#"{"keys":{},"sk-secret-1234":1}"#,
    b"{\"keys\":{\"openai\":\"sk-secret-1234\xff\"}}",
];

#[test]
fn a_damaged_file_is_never_written_over() {
    for bytes in DAMAGED {
        let shown = String::from_utf8_lossy(bytes);
        let app = app();
        app.save(&settings_with(&["mine"])).unwrap();
        std::fs::write(&app.file, bytes).unwrap();
        for (what, result) in [
            ("set", app.set("mine", "sk-mine-1234567890").map(|_| ())),
            ("delete", delete_provider_key(&app.db, &app.store, "mine").map(|_| ())),
            ("removal", app.save(&Settings::default())),
        ] {
            assert_eq!(std::fs::read(&app.file).unwrap(), bytes, "{what} changed {shown}");
            let Err(e) = result else { panic!("{what} took {shown} as read") };
            let message = format!("{e:#}");
            assert!(message.contains("credentials.json"), "{message}");
            assert!(!message.contains("secret"), "{message}");
        }
    }
}

#[test]
fn a_file_that_cannot_be_opened_is_never_written_over() {
    let app = app();
    app.set("openai", "sk-secret-1234").unwrap();
    let bytes = std::fs::read(&app.file).unwrap();
    std::fs::set_permissions(&app.file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let set = app.set("gemini", "sk-mine-1234567890");
    std::fs::set_permissions(&app.file, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(std::fs::read(&app.file).unwrap(), bytes);
    assert!(set.is_err());
}

#[test]
fn a_damaged_file_does_not_stop_settings_that_have_nothing_to_do_with_it() {
    let app = app();
    app.save(&settings_with(&["mine"])).unwrap();
    std::fs::write(&app.file, DAMAGED[3]).unwrap();
    let mut settings = settings_with(&["mine", "another"]);
    settings.just_my_words = true;
    app.save(&settings).unwrap();
    assert!(app.db.get_settings().unwrap().just_my_words);
}

#[test]
fn a_damaged_file_is_named_on_the_providers_that_need_a_key() {
    std::env::set_var("DURAGON_TEST_STATUS_KEY", "sk-from-env-1234");
    let app = app();
    std::fs::write(&app.file, DAMAGED[4]).unwrap();
    let mut providers = Settings::default().resolved_providers();
    providers.push(ProviderConfig { env_var: Some("DURAGON_TEST_STATUS_KEY".into()), ..custom("mine") });
    for (provider, status) in providers.iter().zip(app.store.statuses(&providers)) {
        let one = serde_json::to_value(app.store.status_for(provider)).unwrap();
        let status = serde_json::to_value(status).unwrap();
        assert_eq!(one, status);
        let keyed = matches!(provider.kind, ProviderKind::OpenAi | ProviderKind::Anthropic | ProviderKind::Gemini);
        assert_eq!(status["error"].is_string(), keyed, "{status}");
        assert!(!status.to_string().contains("secret"), "{status}");
        assert_eq!(status["hasEnvKey"], false, "{status}");
    }

    std::fs::remove_file(&app.file).unwrap();
    let well = serde_json::to_value(app.store.status_for(providers.last().unwrap())).unwrap();
    assert_eq!((&well["error"], &well["hasEnvKey"]), (&Value::Null, &json!(true)));
}

async fn asked_with(app: &App, settings: Settings) -> Vec<(String, Value)> {
    app.save(&settings).unwrap();
    std::fs::write(&app.file, DAMAGED[3]).unwrap();
    let note = app.db.create_note(Some("Plants")).unwrap();
    let cap = Arc::new(Capture::default());
    let mut ai = Ai::new(app.db.clone(), cap.clone());
    ai.credentials = Some(app.store.clone());
    ai.bin = Some(fake_claude(app.dir.path(), &json!({"said":true,"quote":"cell","message":"Which cell?"}), &json!({})));
    ai.read(&note.meta.id, "The cell divides.", 0).await.unwrap();
    cap.all()
}

#[tokio::test]
async fn a_damaged_file_never_blocks_a_provider_that_needs_no_key() {
    let events = asked_with(&app(), Settings { learn_about_you: false, ..Settings::default() }).await;
    assert!(events.iter().any(|(name, _)| name == "ai:thread"), "{events:?}");
}

#[tokio::test]
async fn a_call_that_needs_a_key_says_the_file_is_damaged_and_sends_nothing() {
    std::env::set_var("DURAGON_TEST_CALL_KEY", "sk-from-env-1234");
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    server.set_nonblocking(true).unwrap();
    let mut settings = Settings { learn_about_you: false, read_provider: "mine".into(), ..Settings::default() };
    settings.providers.push(ProviderConfig {
        env_var: Some("DURAGON_TEST_CALL_KEY".into()),
        base_url: Some(format!("http://{}/v1", server.local_addr().unwrap())),
        ..custom("mine")
    });
    let app = app();
    let events = tokio::time::timeout(Duration::from_secs(20), asked_with(&app, settings)).await;
    assert!(server.accept().is_err(), "the environment's key was sent");
    let events = events.unwrap();
    let (_, error) = events.iter().find(|(name, _)| name == "ai:error").expect("an error the writer sees");
    let message = error["message"].as_str().unwrap();
    assert!(message.contains("credentials.json"), "{message}");
    assert!(!message.contains("secret"), "{message}");
}
