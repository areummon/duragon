use duragon_core::credentials::{delete_provider_key, save_settings, set_provider_key, CredentialsStore};
use duragon_core::db::Db;
use duragon_core::model::{ProviderConfig, ProviderKind, Settings};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use tempfile::TempDir;

struct App {
    db: Db,
    store: CredentialsStore,
    file: PathBuf,
    _dir: TempDir,
}

fn app() -> App {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("credentials.json");
    App {
        db: Db::open(&dir.path().join("duragon.db")).unwrap(),
        store: CredentialsStore::new(file.clone()),
        file,
        _dir: dir,
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
