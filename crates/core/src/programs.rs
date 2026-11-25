//! The programs the writer's providers run.
use crate::model::{default_providers, ProviderConfig, ProviderKind, Settings};

type Program = (ProviderKind, String, Vec<String>);

fn program(provider: &ProviderConfig) -> Option<Program> {
    let preset = match provider.kind {
        ProviderKind::ClaudeCli => "claude",
        ProviderKind::CodexCli => "codex",
        ProviderKind::AgyAcp | ProviderKind::Acp => "agy",
        _ => return None,
    };
    let command = provider.command.as_deref().map(str::trim).filter(|c| !c.is_empty()).unwrap_or(preset);
    Some((provider.kind, command.to_string(), provider.args.clone()))
}

/// Quoted where it could be read as something else.
fn shown((_, command, args): &Program) -> String {
    let word = |w: &String| match w.chars().all(|c| c.is_ascii_graphic()) && !w.is_empty() {
        true => w.clone(),
        false => format!("{w:?}"),
    };
    std::iter::once(command).chain(args).map(word).collect::<Vec<_>>().join(" ")
}

/// The programs in `settings` that don't come with Duragon and that the stored settings don't have.
pub fn new_programs(stored: Option<&Settings>, settings: &Settings) -> Vec<String> {
    let presets = default_providers();
    let known: Vec<Program> = presets.iter().chain(stored.iter().flat_map(|s| &s.providers)).filter_map(program).collect();
    settings.providers.iter().filter_map(program).filter(|p| !known.contains(p)).map(|p| shown(&p)).collect()
}
