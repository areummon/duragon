//! A regular expression for every name but a few. Go's have no negation.
use std::collections::BTreeSet;

/// Matches, as a whole, each nonempty name that is not one of `names`. The names are plain words.
pub(super) fn all_but(names: &[&str]) -> String {
    let mut prefixes: BTreeSet<&str> = names.iter().flat_map(|n| (0..=n.len()).map(|i| &n[..i])).collect();
    prefixes.insert("");
    let mut parts = Vec::new();
    for prefix in prefixes {
        let next: BTreeSet<char> = names.iter().filter_map(|n| n.strip_prefix(prefix)?.chars().next()).collect();
        if !prefix.is_empty() && !names.contains(&prefix) {
            parts.push(prefix.to_string());
        }
        parts.push(if next.is_empty() {
            format!("{prefix}.+")
        } else {
            format!("{prefix}[^{}].*", next.into_iter().collect::<String>())
        });
    }
    format!("^({})$", parts.join("|"))
}
