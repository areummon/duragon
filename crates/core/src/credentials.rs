//! Local `0600` API key storage (`credentials.json`) with environment-variable fallback.
//! Raw keys stay in Rust; the frontend only receives masked status snapshots.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::db::Db;
use crate::model::{ProviderConfig, ProviderCredentialStatus, Settings};
use crate::private;

#[derive(Debug, Default, Serialize, Deserialize)]
struct FileData {
    #[serde(default)]
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

    fn read_file(&self) -> FileData {
        let Ok(raw) = std::fs::read_to_string(&self.path) else {
            return FileData::default();
        };
        private::file(&self.path);
        serde_json::from_str(&raw).unwrap_or_default()
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
    pub fn resolve(&self, provider_id: &str, env_var: Option<&str>) -> Option<String> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let data = self.read_file();
        if let Some(k) = data.keys.get(provider_id).map(|s| s.trim()).filter(|s| !s.is_empty()) {
            return Some(k.to_string());
        }
        env_var
            .and_then(|name| std::env::var(name).ok())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }

    /// Stores or clears a key for `provider_id`.
    pub fn set_key(&self, provider_id: &str, key: &str) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut data = self.read_file();
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

    /// Drops every key whose provider is not among `providers`.
    pub fn prune(&self, providers: &[ProviderConfig]) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut data = self.read_file();
        let before = data.keys.len();
        data.keys.retain(|id, _| providers.iter().any(|p| p.id == *id));
        if data.keys.len() == before {
            return Ok(());
        }
        self.write_file(&data)
    }

    pub fn status_for(&self, provider: &ProviderConfig) -> ProviderCredentialStatus {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let data = self.read_file();
        status_from_data(&data, provider)
    }

    pub fn statuses(&self, providers: &[ProviderConfig]) -> Vec<ProviderCredentialStatus> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let data = self.read_file();
        providers.iter().map(|p| status_from_data(&data, p)).collect()
    }
}

/// Saves the settings, and drops the keys of providers they no longer have.
pub fn save_settings(db: &Db, store: &CredentialsStore, settings: &Settings) -> Result<()> {
    db.save_settings(settings)?;
    store.prune(&settings.resolved_providers())
}

fn known_provider(db: &Db, id: &str) -> Result<ProviderConfig> {
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

fn status_from_data(data: &FileData, provider: &ProviderConfig) -> ProviderCredentialStatus {
    let stored = data.keys.get(&provider.id).map(|s| s.trim()).filter(|s| !s.is_empty());
    let has_env_key = provider
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

        assert_eq!(store.resolve("openai", None), None);
        store.set_key("openai", "sk-proj-1234567890abcdef").unwrap();
        assert_eq!(store.resolve("openai", None).as_deref(), Some("sk-proj-1234567890abcdef"));

        let st = store.status_for(&openai);
        assert!(st.has_stored_key);
        assert_eq!(st.masked_key.as_deref(), Some("sk-p…cdef"));

        store.delete_key("openai").unwrap();
        assert_eq!(store.resolve("openai", None), None);
        assert!(!store.status_for(&openai).has_stored_key);
    }
}
