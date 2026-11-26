//! Configurable web search providers (Tavily, Brave, Exa, Firecrawl, Ollama hosted search).
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;
use std::time::Duration;

use crate::credentials::{env_key, CredentialsStore};
use crate::model::SearchEngine;

pub use super::web_fetch::{fetch_web_page, is_public_ip, run_web_fetch_tool};

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()
        .expect("reqwest search client")
});

const MAX_SNIPPET_CHARS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResolution {
    pub available: bool,
    pub engine: Option<SearchEngine>,
    pub reason: String,
    pub chosen: Option<(SearchEngine, String)>,
}

/// Pure engine resolution: explicit pin wins (and never silently falls back if its key is missing);
/// absent pin selects the first keyed engine in `SearchEngine::ALL` order.
pub fn resolve_search(
    pinned: Option<SearchEngine>,
    keys: &[(SearchEngine, Option<String>)],
) -> SearchResolution {
    let key_for = |eng: SearchEngine| -> Option<String> {
        keys.iter()
            .find(|(e, _)| *e == eng)
            .and_then(|(_, k)| k.as_deref())
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(String::from)
    };

    if let Some(eng) = pinned {
        return match key_for(eng) {
            Some(key) => SearchResolution {
                available: true,
                engine: Some(eng),
                reason: format!("Using {}", eng.label()),
                chosen: Some((eng, key)),
            },
            None => SearchResolution {
                available: false,
                engine: None,
                reason: format!("{} is selected, but its API key is missing", eng.label()),
                chosen: None,
            },
        };
    }

    for eng in SearchEngine::ALL {
        if let Some(key) = key_for(eng) {
            return SearchResolution {
                available: true,
                engine: Some(eng),
                reason: format!("Using {} (automatic)", eng.label()),
                chosen: Some((eng, key)),
            };
        }
    }

    SearchResolution {
        available: false,
        engine: None,
        reason: "No search engine key set".into(),
        chosen: None,
    }
}

#[derive(Clone)]
pub struct SearchClient {
    engine: SearchEngine,
    api_key: String,
    base_url: Option<String>,
}

impl SearchClient {
    pub fn new(engine: SearchEngine, api_key: String) -> Self {
        Self { engine, api_key, base_url: None }
    }

    pub fn with_base_url(engine: SearchEngine, api_key: String, base_url: String) -> Self {
        Self { engine, api_key, base_url: Some(base_url) }
    }

    pub fn engine(&self) -> SearchEngine {
        self.engine
    }

    pub(super) fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn resolve(pinned: Option<SearchEngine>, store: Option<&CredentialsStore>) -> Option<Self> {
        let keys = match store {
            Some(s) => s.resolve_search_keys().ok()?,
            None => SearchEngine::ALL
                .into_iter()
                .map(|eng| (eng, env_key(eng.env_var())))
                .collect(),
        };
        resolve_search(pinned, &keys)
            .chosen
            .map(|(eng, key)| Self::new(eng, key))
    }

    fn base(&self, default: &str) -> String {
        self.base_url
            .as_deref()
            .unwrap_or(default)
            .trim_end_matches('/')
            .to_string()
    }

    async fn send_search_json(
        &self,
        req: RequestBuilder,
        error_detail: Option<fn(&Value) -> Option<&str>>,
    ) -> Result<Value, String> {
        let resp = req
            .send()
            .await
            .map_err(|e| format!("{} search request failed: {}", self.engine.label(), e.without_url()))?;
        let status = resp.status();
        if !status.is_success() {
            let detail = if let Some(extract) = error_detail {
                resp.text()
                    .await
                    .ok()
                    .and_then(|body| serde_json::from_str::<Value>(&body).ok())
                    .and_then(|v| extract(&v).map(|s| s.chars().take(200).collect::<String>()))
            } else {
                None
            };
            return Err(match detail {
                Some(msg) if !msg.trim().is_empty() => {
                    format!("{} search API {}: {msg}", self.engine.label(), status.as_u16())
                }
                _ => format!("{} search API {}", self.engine.label(), status.as_u16()),
            });
        }
        resp.json::<Value>()
            .await
            .map_err(|_| invalid_response(self.engine))
    }

    pub async fn search(&self, query: &str, count: usize) -> Result<Vec<SearchHit>, String> {
        match self.engine {
            SearchEngine::Tavily => self.search_tavily(query, count).await,
            SearchEngine::Brave => self.search_brave(query, count).await,
            SearchEngine::Exa => self.search_exa(query, count).await,
            SearchEngine::Firecrawl => self.search_firecrawl(query, count).await,
            SearchEngine::Ollama => self.search_ollama(query, count).await,
        }
    }

    async fn search_tavily(&self, query: &str, count: usize) -> Result<Vec<SearchHit>, String> {
        let url = format!("{}/search", self.base("https://api.tavily.com"));
        let req = CLIENT
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&json!({ "query": query, "max_results": count.clamp(1, 20) }));
        let data = self
            .send_search_json(req, Some(|v| v["detail"]["error"].as_str()))
            .await?;
        collect_hits(data.get("results"), "content", self.engine)
    }

    async fn search_brave(&self, query: &str, count: usize) -> Result<Vec<SearchHit>, String> {
        let url = format!("{}/res/v1/web/search", self.base("https://api.search.brave.com"));
        let count_str = count.clamp(1, 20).to_string();
        let req = CLIENT
            .get(&url)
            .query(&[("q", query), ("count", count_str.as_str())])
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json");
        let data = self.send_search_json(req, None).await?;
        collect_hits(data.get("web").and_then(|w| w.get("results")), "description", self.engine)
    }

    async fn search_exa(&self, query: &str, count: usize) -> Result<Vec<SearchHit>, String> {
        let url = format!("{}/search", self.base("https://api.exa.ai"));
        let req = CLIENT
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&json!({
                "query": query,
                "type": "auto",
                "numResults": count.clamp(1, 100),
                "contents": { "highlights": true }
            }));
        let data = self.send_search_json(req, None).await?;
        let results = data
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid_response(self.engine))?;
        Ok(results
            .iter()
            .filter_map(|r| {
                let highlights = r
                    .get("highlights")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let snippet = if !highlights.trim().is_empty() {
                    highlights
                } else {
                    r.get("text").and_then(Value::as_str).unwrap_or("").to_string()
                };
                make_hit(
                    r.get("title").and_then(Value::as_str),
                    r.get("url").and_then(Value::as_str),
                    &snippet,
                )
            })
            .collect())
    }

    async fn search_firecrawl(&self, query: &str, count: usize) -> Result<Vec<SearchHit>, String> {
        let url = format!("{}/v2/search", self.base("https://api.firecrawl.dev"));
        let req = CLIENT
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&json!({ "query": query, "limit": count.clamp(1, 100) }));
        let data = self.send_search_json(req, None).await?;
        if data.get("success").and_then(Value::as_bool) != Some(true) {
            return Err(invalid_response(self.engine));
        }
        collect_hits(data.get("data").and_then(|d| d.get("web")), "description", self.engine)
    }

    async fn search_ollama(&self, query: &str, count: usize) -> Result<Vec<SearchHit>, String> {
        let url = format!("{}/api/web_search", self.base("https://ollama.com"));
        let req = CLIENT
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&json!({ "query": query, "max_results": count.clamp(1, 10) }));
        let data = self.send_search_json(req, None).await?;
        collect_hits(data.get("results"), "content", self.engine)
    }
}

fn collect_hits(
    results: Option<&Value>,
    snippet_field: &str,
    engine: SearchEngine,
) -> Result<Vec<SearchHit>, String> {
    let arr = results
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_response(engine))?;
    Ok(arr
        .iter()
        .filter_map(|r| {
            make_hit(
                r.get("title").and_then(Value::as_str),
                r.get("url").and_then(Value::as_str),
                r.get(snippet_field).and_then(Value::as_str).unwrap_or(""),
            )
        })
        .collect())
}

fn invalid_response(engine: SearchEngine) -> String {
    format!("{} search API returned an invalid response", engine.label())
}

fn make_hit(title: Option<&str>, url: Option<&str>, snippet: &str) -> Option<SearchHit> {
    let url = url?.trim();
    if url.is_empty() {
        return None;
    }
    let parsed = reqwest::Url::parse(url).ok()?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return None;
    }
    let title = title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or(url)
        .to_string();
    let collapsed = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
    let snippet: String = collapsed.chars().take(MAX_SNIPPET_CHARS).collect();
    Some(SearchHit { title, url: url.to_string(), snippet })
}

/// Runs the `web_search` tool call for an AI provider loop or MCP server.
pub async fn run_web_search_tool(client: &SearchClient, args: &Value) -> Result<String, String> {
    let query = args.get("query").and_then(Value::as_str).map(str::trim).unwrap_or("");
    if query.is_empty() {
        return Err("web_search failed: query is empty".into());
    }
    let masked_query = super::veil::mask_with(query, &[client.api_key()]);
    let masked_query = masked_query.trim();
    if masked_query.is_empty() {
        return Err("web_search failed: query is empty".into());
    }
    let count = args.get("count").and_then(Value::as_u64).map(|n| n as usize).unwrap_or(5);
    match client.search(masked_query, count).await {
        Ok(hits) => Ok(serde_json::to_string(&hits).unwrap_or_else(|_| "[]".into())),
        Err(err) => Err(format!("web_search failed: {err}")),
    }
}
