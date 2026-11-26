//! Local `0600` API key storage (`credentials.json`) with environment-variable fallback.
//! Raw keys stay in Rust; the frontend only receives masked status snapshots.
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::ai::search::resolve_search;
use crate::consent::{asks, Asks};
use crate::db::Db;
use crate::model::{
    ProviderConfig, ProviderCredentialStatus, SearchEngine, SearchEngineCredentialStatus, SearchStatus, Settings,
};
use crate::private;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileData {
    keys: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct CredentialsStore {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl CredentialsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path, lock: Arc::new(Mutex::new(())) }
    }

    /// Only a missing file holds no keys. The parser's own message is left out, because it can quote a key.
    fn read_file(&self) -> Result<FileData> {
        let unreadable = |why: String| anyhow!("The saved API keys in {} can't be read: {why}", self.path.display());
        let raw = match std::fs::read(&self.path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(FileData::default()),
            Err(e) => return Err(unreadable(e.to_string())),
        };
        private::file(&self.path);
        serde_json::from_slice(&raw)
            .map_err(|_| unreadable("the file is damaged. Repair it, or delete it and add the keys again.".into()))
    }

    fn write_file(&self, data: &FileData) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            private::dir(parent)?;
        }
        let json = serde_json::to_string_pretty(data)?;
        let tmp = self.path.with_extension("json.tmp");
        {
            let mut f = private::options().create(true).truncate(true).open(&tmp)?;
            f.write_all(json.as_bytes())?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, &self.path)?;
        private::file(&self.path);
        Ok(())
    }

    /// Resolves an API key: stored key in `credentials.json` first, then `env_var` fallback.
    /// An unreadable file may hold a key that outranks the environment's, so it is an error.
    pub fn resolve(&self, provider_id: &str, env_var: Option<&str>) -> Result<Option<String>> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let data = self.read_file()?;
        if let Some(k) = data.keys.get(provider_id).map(|s| s.trim()).filter(|s| !s.is_empty()) {
            return Ok(Some(k.to_string()));
        }
        Ok(env_var
            .and_then(|name| std::env::var(name).ok())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()))
    }

    /// The key a call to `provider` carries. A server the writer runs works without one, so a damaged file can't stop it.
    pub fn resolve_for(&self, provider: &ProviderConfig) -> Result<Option<String>> {
        let key = || self.resolve(&provider.id, provider.env_var.as_deref());
        if provider.kind.is_local_server() {
            Ok(key().unwrap_or_default())
        } else if provider.kind.needs_key() {
            key()
        } else {
            Ok(None)
        }
    }

    /// Stores or clears a key for `provider_id`.
    pub fn set_key(&self, provider_id: &str, key: &str) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut data = self.read_file()?;
        let trimmed = key.trim();
        if trimmed.is_empty() {
            data.keys.remove(provider_id);
        } else {
            data.keys.insert(provider_id.to_string(), trimmed.to_string());
        }
        self.write_file(&data)
    }

    pub fn delete_key(&self, provider_id: &str) -> Result<()> {
        self.set_key(provider_id, "")
    }

    pub fn forget(&self, provider_ids: &[String]) -> Result<()> {
        if provider_ids.is_empty() {
            return Ok(());
        }
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut data = self.read_file()?;
        let before = data.keys.len();
        data.keys.retain(|id, _| !provider_ids.contains(id));
        if data.keys.len() == before {
            return Ok(());
        }
        self.write_file(&data)
    }

    pub fn status_for(&self, provider: &ProviderConfig) -> ProviderCredentialStatus {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let data = self.read_file().map_err(|e| format!("{e:#}"));
        status_from_data(&data, provider)
    }

    pub fn statuses(&self, providers: &[ProviderConfig]) -> Vec<ProviderCredentialStatus> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let data = self.read_file().map_err(|e| format!("{e:#}"));
        providers.iter().map(|p| status_from_data(&data, p)).collect()
    }

    pub fn resolve_search_key(&self, engine: SearchEngine) -> Result<Option<String>> {
        self.resolve(&engine.credential_key(), Some(engine.env_var()))
    }

    pub fn search_statuses(&self) -> Vec<SearchEngineCredentialStatus> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let data = self.read_file().map_err(|e| format!("{e:#}"));
        SearchEngine::ALL
            .into_iter()
            .map(|engine| search_status_from_data(&data, engine))
            .collect()
    }

    pub fn set_search_key(&self, engine: SearchEngine, key: &str) -> Result<()> {
        let valid = validate_search_key(engine, key)?;
        self.set_key(&engine.credential_key(), &valid)
    }

    pub fn delete_search_key(&self, engine: SearchEngine) -> Result<()> {
        self.delete_key(&engine.credential_key())
    }
}

pub fn validate_search_key(engine: SearchEngine, raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("{} API key cannot be empty", engine.label());
    }
    if trimmed.chars().any(char::is_whitespace) {
        bail!("{} API key must not contain whitespace", engine.label());
    }
    match engine {
        SearchEngine::Tavily => {
            if !trimmed.starts_with("tvly-") || trimmed.len() < 20 {
                bail!("Tavily API key must start with \"tvly-\" and be at least 20 characters");
            }
        }
        SearchEngine::Firecrawl => {
            if !trimmed.starts_with("fc-") || trimmed.len() < 20 {
                bail!("Firecrawl API key must start with \"fc-\" and be at least 20 characters");
            }
        }
        SearchEngine::Brave => {
            if trimmed.len() < 20 {
                bail!("Brave Search API key must be at least 20 characters");
            }
        }
        SearchEngine::Exa | SearchEngine::Ollama => {
            if trimmed.len() < 12 {
                bail!("{} API key must be at least 12 characters", engine.label());
            }
        }
    }
    Ok(trimmed.to_string())
}

pub fn search_status(db: &Db, store: &CredentialsStore) -> Result<SearchStatus> {
    let pinned = db.get_settings()?.web_search_engine;
    let engines = store.search_statuses();
    let keys: Vec<(SearchEngine, Option<String>)> = SearchEngine::ALL
        .into_iter()
        .map(|eng| (eng, store.resolve_search_key(eng).ok().flatten()))
        .collect();
    let (available, engine, reason, _) = resolve_search(pinned, &keys);
    Ok(SearchStatus { available, engine, pinned, reason, engines })
}

pub fn set_search_key(
    db: &Db,
    store: &CredentialsStore,
    engine: SearchEngine,
    key: &str,
) -> Result<SearchStatus> {
    store.set_search_key(engine, key)?;
    search_status(db, store)
}

pub fn delete_search_key(
    db: &Db,
    store: &CredentialsStore,
    engine: SearchEngine,
) -> Result<SearchStatus> {
    store.delete_search_key(engine)?;
    search_status(db, store)
}

fn search_status_from_data(data: &Result<FileData, String>, engine: SearchEngine) -> SearchEngineCredentialStatus {
    let error = data.as_ref().err().cloned();
    let stored = data
        .as_ref()
        .ok()
        .and_then(|d| d.keys.get(&engine.credential_key()))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let has_env_key = error.is_none()
        && std::env::var(engine.env_var())
            .ok()
            .is_some_and(|v| !v.trim().is_empty());
    SearchEngineCredentialStatus {
        engine,
        label: engine.label().to_string(),
        env_var: engine.env_var().to_string(),
        docs_url: engine.docs_url().to_string(),
        has_stored_key: stored.is_some(),
        masked_key: stored.map(mask_key),
        has_env_key,
        error,
    }
}

/// Saves the settings, after forgetting the keys of the providers they remove.
/// Saved settings that can't be parsed remove none, because their providers are unknown.
///
/// `allow` asks the writer about the programs and addresses the settings add. It must not be the page that answers:
/// markup that got past the cleaning could otherwise start anything, or send the notes and keys anywhere.
pub fn save_settings(
    db: &Db,
    store: &CredentialsStore,
    settings: &Settings,
    allow: impl FnOnce(&Asks) -> bool,
) -> Result<()> {
    let asks = asks(db.stored_settings()?.as_ref(), settings);
    if !asks.is_empty() && !allow(&asks) {
        bail!("The change was not allowed");
    }
    let kept = settings.resolved_providers();
    let stored = db.stored_settings()?.map(|s| s.resolved_providers()).unwrap_or_default();
    let removed: Vec<String> =
        stored.into_iter().map(|p| p.id).filter(|id| kept.iter().all(|p| p.id != *id)).collect();
    store.forget(&removed).context("The settings were not saved, because a removed provider's API key could not be deleted")?;
    db.save_settings(settings)
}

pub(crate) fn known_provider(db: &Db, id: &str) -> Result<ProviderConfig> {
    let providers = db.get_settings()?.resolved_providers();
    providers.into_iter().find(|p| p.id == id).with_context(|| format!("there is no provider \"{id}\""))
}

pub fn set_provider_key(
    db: &Db,
    store: &CredentialsStore,
    provider_id: &str,
    key: &str,
) -> Result<ProviderCredentialStatus> {
    let provider = known_provider(db, provider_id)?;
    store.set_key(provider_id, key)?;
    Ok(store.status_for(&provider))
}

pub fn delete_provider_key(db: &Db, store: &CredentialsStore, provider_id: &str) -> Result<ProviderCredentialStatus> {
    let provider = known_provider(db, provider_id)?;
    store.delete_key(provider_id)?;
    Ok(store.status_for(&provider))
}

fn status_from_data(data: &Result<FileData, String>, provider: &ProviderConfig) -> ProviderCredentialStatus {
    let error = data.as_ref().err().filter(|_| provider.kind.needs_key()).cloned();
    let stored = data
        .as_ref()
        .ok()
        .and_then(|d| d.keys.get(&provider.id))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let has_env_key = error.is_none()
        && provider
            .env_var
            .as_deref()
            .and_then(|v| std::env::var(v).ok())
            .is_some_and(|v| !v.trim().is_empty());
    ProviderCredentialStatus {
        provider_id: provider.id.clone(),
        has_stored_key: stored.is_some(),
        masked_key: stored.map(mask_key),
        env_var: provider.env_var.clone(),
        has_env_key,
        error,
    }
}

pub fn mask_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        return "••••".into();
    }
    let prefix: String = chars.iter().take(4).collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{prefix}…{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::default_providers;

    #[test]
    fn round_trip_and_mask() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialsStore::new(dir.path().join("credentials.json"));
        let openai = default_providers().into_iter().find(|p| p.id == "openai").unwrap();

        assert_eq!(store.resolve("openai", None).unwrap(), None);
        store.set_key("openai", "sk-proj-1234567890abcdef").unwrap();
        assert_eq!(store.resolve("openai", None).unwrap().as_deref(), Some("sk-proj-1234567890abcdef"));

        let st = store.status_for(&openai);
        assert!(st.has_stored_key);
        assert_eq!(st.masked_key.as_deref(), Some("sk-p…cdef"));

        store.delete_key("openai").unwrap();
        assert_eq!(store.resolve("openai", None).unwrap(), None);
        assert!(!store.status_for(&openai).has_stored_key);
    }
}
