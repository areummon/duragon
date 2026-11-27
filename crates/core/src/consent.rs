//! What saved settings would newly let Duragon do: run a program, or send notes somewhere.
use crate::model::{default_providers, ProviderConfig, ProviderKind, Settings};

#[derive(Debug, Default, PartialEq)]
pub struct Asks {
    pub programs: Vec<String>,
    pub addresses: Vec<String>,
}

impl Asks {
    pub fn is_empty(&self) -> bool {
        self.programs.is_empty() && self.addresses.is_empty()
    }
}

#[derive(PartialEq)]
enum Reach {
    Program(ProviderKind, String, Vec<String>),
    /// Each provider's own, with where its key comes from, so no key can be turned to another's address.
    Address { provider: String, url: String, key_from: Option<String> },
}

fn reach(provider: &ProviderConfig) -> Reach {
    let preset = match provider.kind {
        ProviderKind::ClaudeCli => "claude",
        ProviderKind::CodexCli => "codex",
        ProviderKind::AgyAcp | ProviderKind::Acp => "agy",
        _ => {
            return Reach::Address {
                provider: provider.id.clone(),
                url: provider.base_url.as_deref().unwrap_or_default().trim().to_string(),
                key_from: provider.env_var.clone(),
            }
        }
    };
    let command = provider.command.as_deref().map(str::trim).filter(|c| !c.is_empty()).unwrap_or(preset);
    Reach::Program(provider.kind, command.to_string(), provider.args.clone())
}

/// Quoted where it could be read as something else.
pub fn plain(word: &str) -> String {
    match word.chars().all(|c| c.is_ascii_graphic()) && !word.is_empty() {
        true => word.to_string(),
        false => format!("{word:?}"),
    }
}

/// What the settings ask for that does not come with Duragon and is not in the stored ones.
pub fn asks(stored: Option<&Settings>, settings: &Settings) -> Asks {
    let presets = default_providers();
    let known: Vec<Reach> = presets.iter().chain(stored.iter().flat_map(|s| &s.providers)).map(reach).collect();
    let mut asks = Asks::default();
    for provider in &settings.providers {
        let new = reach(provider);
        if known.contains(&new) {
            continue;
        }
        match new {
            Reach::Program(_, command, args) => {
                let words: Vec<String> = std::iter::once(&command).chain(&args).map(|w| plain(w)).collect();
                asks.programs.push(words.join(" "));
            }
            Reach::Address { url, key_from, .. } => {
                // The host is what counts, and an address can be written to hide it.
                let host = reqwest::Url::parse(&url).ok().and_then(|u| u.host_str().map(String::from));
                let mut line = format!("{}, for {}", plain(host.as_deref().unwrap_or(&url)), plain(&provider.name));
                if let Some(variable) = key_from {
                    line.push_str(&format!(", with the key in {}", plain(&variable)));
                }
                asks.addresses.push(format!("{line}\n{}", plain(&url)));
            }
        }
    }
    asks
}
