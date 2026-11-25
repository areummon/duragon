//! Masks the emails and API keys in what a model is sent, and puts them back in what it answers.
use serde_json::Value;
use std::ops::Range;

/// How API keys of the services people use start.
const KEY_STARTS: [&str; 19] = [
    "sk-", "sk_", "pk_", "rk_", "AIza", "AKIA", "ASIA", "ya29", "ghp_", "gho_", "ghs_", "github_pat_", "glpat-", "xoxb-",
    "xoxp-", "hf_", "gsk_", "pplx-", "nvapi-",
];
const KEY_LEAST: usize = 16;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Email,
    Key,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Email => "email",
            Kind::Key => "key",
        }
    }
}

fn email_around(text: &str, at: usize) -> Option<Range<usize>> {
    let local = |c: char| c.is_ascii_alphanumeric() || "._%+-".contains(c);
    let domain = |c: char| c.is_ascii_alphanumeric() || ".-".contains(c);
    let before = &text[..at];
    let start = before.len() - before.chars().rev().take_while(|c| local(*c)).count();
    let after = &text[at + 1..];
    let end = after.chars().take_while(|c| domain(*c)).count();
    let name = before[start..].trim_start_matches('.');
    let host = after[..end].trim_end_matches(['.', '-']);
    let (_, top) = host.rsplit_once('.')?;
    (!name.is_empty() && top.len() >= 2 && top.chars().all(|c| c.is_ascii_alphabetic()))
        .then(|| at - name.len()..at + 1 + host.len())
}

fn private_parts(text: &str, secret: Option<&str>) -> Vec<(Range<usize>, Kind)> {
    let mut parts: Vec<(Range<usize>, Kind)> = vec![];
    if let Some(secret) = secret {
        parts.extend(text.match_indices(secret).map(|(i, s)| (i..i + s.len(), Kind::Key)));
    }
    let word = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    let mut from = 0;
    for run in text.split(|c| !word(c)) {
        let start = from;
        from += run.len() + text[from + run.len()..].chars().next().map_or(0, char::len_utf8);
        let key = run.len() >= KEY_LEAST
            && KEY_STARTS.iter().any(|s| run.starts_with(s))
            && run.chars().any(|c| c.is_ascii_digit());
        if key {
            parts.push((start..start + run.len(), Kind::Key));
        }
    }
    parts.extend(text.match_indices('@').filter_map(|(at, _)| Some((email_around(text, at)?, Kind::Email))));
    parts.sort_by_key(|(r, _)| (r.start, std::cmp::Reverse(r.end)));
    let mut end = 0;
    parts.retain(|(r, _)| {
        let apart = r.start >= end;
        end = end.max(r.end);
        apart
    });
    parts
}

fn replaced(text: &str, secret: Option<&str>, mut by: impl FnMut(&str, Kind) -> String) -> String {
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for (range, kind) in private_parts(text, secret) {
        out.push_str(&text[from..range.start]);
        out.push_str(&by(&text[range.clone()], kind));
        from = range.end;
    }
    out.push_str(&text[from..]);
    out
}

/// Masks for good, where the answer can't be mended.
pub(super) fn mask(text: &str) -> String {
    replaced(text, None, |_, kind| format!("[{}]", kind.name()))
}

/// What one call keeps from a model, and what stands for it.
pub(super) struct Veil {
    secret: Option<String>,
    hidden: Vec<(String, String)>,
}

impl Veil {
    /// `secret` is the key the call itself carries, however it looks.
    pub(super) fn new(secret: Option<&str>) -> Self {
        Self { secret: secret.map(str::trim).filter(|s| s.len() >= 8).map(String::from), hidden: vec![] }
    }

    pub(super) fn cover(&mut self, text: &str) -> String {
        let hidden = &mut self.hidden;
        replaced(text, self.secret.as_deref(), |found, kind| {
            if let Some((_, stand_in)) = hidden.iter().find(|(f, _)| f == found) {
                return stand_in.clone();
            }
            let stand_in = format!("⟦{} {}⟧", kind.name(), hidden.len() + 1);
            hidden.push((found.to_string(), stand_in.clone()));
            stand_in
        })
    }

    /// Tells the model what it will see, when anything is covered.
    pub(super) fn note(&self) -> &'static str {
        if self.hidden.is_empty() {
            ""
        } else {
            "\n\nText in ⟦ ⟧ stands for an email or key that is kept from you. Quote it exactly as it is, and don't guess it."
        }
    }

    pub(super) fn uncover(&self, value: Value) -> Value {
        match value {
            Value::String(s) => {
                Value::String(self.hidden.iter().fold(s, |s, (found, stand_in)| s.replace(stand_in.as_str(), found)))
            }
            Value::Array(all) => Value::Array(all.into_iter().map(|v| self.uncover(v)).collect()),
            Value::Object(all) => Value::Object(all.into_iter().map(|(k, v)| (k, self.uncover(v))).collect()),
            other => other,
        }
    }
}
