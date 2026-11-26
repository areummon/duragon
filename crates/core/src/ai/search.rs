//! Configurable web search providers (Tavily, Brave, Exa, Firecrawl, Ollama hosted search).
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::{IpAddr, SocketAddr};
use std::sync::LazyLock;
use std::time::Duration;

use crate::credentials::CredentialsStore;
use crate::model::SearchEngine;

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

/// Pure engine resolution: explicit pin wins (and never silently falls back if its key is missing);
/// absent pin selects the first keyed engine in `SearchEngine::ALL` order.
pub fn resolve_search(
    pinned: Option<SearchEngine>,
    keys: &[(SearchEngine, Option<String>)],
) -> (bool, Option<SearchEngine>, String, Option<(SearchEngine, String)>) {
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
            Some(key) => (true, Some(eng), format!("Using {}", eng.label()), Some((eng, key))),
            None => (
                false,
                None,
                format!("{} is selected, but its API key is missing", eng.label()),
                None,
            ),
        };
    }

    for eng in SearchEngine::ALL {
        if let Some(key) = key_for(eng) {
            return (
                true,
                Some(eng),
                format!("Using {} (automatic)", eng.label()),
                Some((eng, key)),
            );
        }
    }

    (false, None, "No search engine key set".into(), None)
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
        let keys: Vec<(SearchEngine, Option<String>)> = SearchEngine::ALL
            .into_iter()
            .map(|eng| {
                let key = match store {
                    Some(s) => s.resolve_search_key(eng).ok().flatten(),
                    None => std::env::var(eng.env_var())
                        .ok()
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty()),
                };
                (eng, key)
            })
            .collect();
        let (_, _, _, chosen) = resolve_search(pinned, &keys);
        chosen.map(|(eng, key)| Self::new(eng, key))
    }

    fn base(&self, default: &str) -> String {
        self.base_url
            .as_deref()
            .unwrap_or(default)
            .trim_end_matches('/')
            .to_string()
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
        let resp = CLIENT
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&json!({ "query": query, "max_results": count.clamp(1, 20) }))
            .send()
            .await
            .map_err(|e| format!("Tavily search request failed: {}", e.without_url()))?;
        let status = resp.status();
        if !status.is_success() {
            let detail = resp
                .text()
                .await
                .ok()
                .and_then(|body| serde_json::from_str::<Value>(&body).ok())
                .and_then(|v| v["detail"]["error"].as_str().map(|s| s.chars().take(200).collect::<String>()));
            return Err(match detail {
                Some(msg) if !msg.trim().is_empty() => format!("Tavily search API {}: {msg}", status.as_u16()),
                _ => format!("Tavily search API {}", status.as_u16()),
            });
        }
        let data = parse_json(resp, self.engine).await?;
        let results = data
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid_response(self.engine))?;
        Ok(results
            .iter()
            .filter_map(|r| {
                make_hit(
                    r.get("title").and_then(Value::as_str),
                    r.get("url").and_then(Value::as_str),
                    r.get("content").and_then(Value::as_str).unwrap_or(""),
                )
            })
            .collect())
    }

    async fn search_brave(&self, query: &str, count: usize) -> Result<Vec<SearchHit>, String> {
        let url = format!("{}/res/v1/web/search", self.base("https://api.search.brave.com"));
        let count_str = count.clamp(1, 20).to_string();
        let resp = CLIENT
            .get(&url)
            .query(&[("q", query), ("count", count_str.as_str())])
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Brave Search request failed: {}", e.without_url()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("{} search API {}", self.engine.label(), status.as_u16()));
        }
        let data = parse_json(resp, self.engine).await?;
        let results = data
            .get("web")
            .and_then(|w| w.get("results"))
            .and_then(Value::as_array)
            .ok_or_else(|| invalid_response(self.engine))?;
        Ok(results
            .iter()
            .filter_map(|r| {
                make_hit(
                    r.get("title").and_then(Value::as_str),
                    r.get("url").and_then(Value::as_str),
                    r.get("description").and_then(Value::as_str).unwrap_or(""),
                )
            })
            .collect())
    }

    async fn search_exa(&self, query: &str, count: usize) -> Result<Vec<SearchHit>, String> {
        let url = format!("{}/search", self.base("https://api.exa.ai"));
        let resp = CLIENT
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&json!({
                "query": query,
                "type": "auto",
                "numResults": count.clamp(1, 100),
                "contents": { "highlights": true }
            }))
            .send()
            .await
            .map_err(|e| format!("Exa search request failed: {}", e.without_url()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("{} search API {}", self.engine.label(), status.as_u16()));
        }
        let data = parse_json(resp, self.engine).await?;
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
        let resp = CLIENT
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&json!({ "query": query, "limit": count.clamp(1, 100) }))
            .send()
            .await
            .map_err(|e| format!("Firecrawl search request failed: {}", e.without_url()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("{} search API {}", self.engine.label(), status.as_u16()));
        }
        let data = parse_json(resp, self.engine).await?;
        if data.get("success").and_then(Value::as_bool) != Some(true) {
            return Err(invalid_response(self.engine));
        }
        let results = data
            .get("data")
            .and_then(|d| d.get("web"))
            .and_then(Value::as_array)
            .ok_or_else(|| invalid_response(self.engine))?;
        Ok(results
            .iter()
            .filter_map(|r| {
                make_hit(
                    r.get("title").and_then(Value::as_str),
                    r.get("url").and_then(Value::as_str),
                    r.get("description").and_then(Value::as_str).unwrap_or(""),
                )
            })
            .collect())
    }

    async fn search_ollama(&self, query: &str, count: usize) -> Result<Vec<SearchHit>, String> {
        let url = format!("{}/api/web_search", self.base("https://ollama.com"));
        let resp = CLIENT
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&json!({ "query": query, "max_results": count.clamp(1, 10) }))
            .send()
            .await
            .map_err(|e| format!("Ollama search request failed: {}", e.without_url()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("{} search API {}", self.engine.label(), status.as_u16()));
        }
        let data = parse_json(resp, self.engine).await?;
        let results = data
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid_response(self.engine))?;
        Ok(results
            .iter()
            .filter_map(|r| {
                make_hit(
                    r.get("title").and_then(Value::as_str),
                    r.get("url").and_then(Value::as_str),
                    r.get("content").and_then(Value::as_str).unwrap_or(""),
                )
            })
            .collect())
    }
}

fn invalid_response(engine: SearchEngine) -> String {
    format!("{} search API returned an invalid response", engine.label())
}

async fn parse_json(resp: reqwest::Response, engine: SearchEngine) -> Result<Value, String> {
    resp.json::<Value>().await.map_err(|_| invalid_response(engine))
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

/// Runs the `web_search` tool call for an AI provider loop. Never panics or aborts the loop.
pub async fn run_web_search_tool(client: &SearchClient, args: &Value) -> String {
    let query = args.get("query").and_then(Value::as_str).map(str::trim).unwrap_or("");
    if query.is_empty() {
        return "web_search failed: query is empty".into();
    }
    let masked_query = super::veil::mask_with(query, &[client.api_key()]);
    let masked_query = masked_query.trim();
    if masked_query.is_empty() {
        return "web_search failed: query is empty".into();
    }
    let count = args.get("count").and_then(Value::as_u64).map(|n| n as usize).unwrap_or(5);
    match client.search(masked_query, count).await {
        Ok(hits) => serde_json::to_string(&hits).unwrap_or_else(|_| "[]".into()),
        Err(err) => format!("web_search failed: {err}"),
    }
}

const MAX_FETCH_BYTES: usize = 2 * 1024 * 1024;
const MAX_FETCH_CHARS: usize = 24_000;
const MAX_FETCH_REDIRECTS: usize = 3;

/// Returns `true` only if `ip` is a globally routable public address.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, c, _] = v4.octets();
            if a == 0 // 0.0.0.0/8
                || a == 10 // 10.0.0.0/8
                || (a == 100 && (64..=127).contains(&b)) // 100.64.0.0/10 (CGNAT)
                || a == 127 // 127.0.0.0/8 (loopback)
                || (a == 169 && b == 254) // 169.254.0.0/16 (link-local & cloud metadata)
                || (a == 172 && (16..=31).contains(&b)) // 172.16.0.0/12
                || (a == 192 && b == 0 && c == 0) // 192.0.0.0/24
                || (a == 192 && b == 0 && c == 2) // 192.0.2.0/24
                || (a == 192 && b == 88 && c == 99) // 192.88.99.0/24
                || (a == 192 && b == 168) // 192.168.0.0/16
                || (a == 198 && (b == 18 || b == 19)) // 198.18.0.0/15
                || (a == 198 && b == 51 && c == 100) // 198.51.100.0/24
                || (a == 203 && b == 0 && c == 113) // 203.0.113.0/24
                || a >= 224 // 224.0.0.0/4 (multicast) & 240.0.0.0/4 (reserved/broadcast)
            {
                return false;
            }
            true
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            if v6.is_unspecified() || v6.is_loopback() || v6.is_multicast() {
                return false;
            }
            let s = v6.segments();
            if (s[0] & 0xfe00) == 0xfc00 // fc00::/7 (ULA)
                || (s[0] & 0xffc0) == 0xfe80 // fe80::/10 (link-local)
                || (s[0] == 0x0064 && s[1] == 0xff9b && s[2] == 0 && s[3] == 0 && s[4] == 0 && s[5] == 0) // 64:ff9b::/96
                || (s[0] == 0x0064 && s[1] == 0xff9b && s[2] == 0x0001) // 64:ff9b:1::/48
                || (s[0] == 0x0100 && s[1] == 0 && s[2] == 0 && s[3] == 0) // 100::/64
                || (s[0] == 0x2001 && s[1] == 0x0db8) // 2001:db8::/32
                || s[0] == 0x2002 // 2002::/16
            {
                return false;
            }
            true
        }
    }
}

fn is_blocked_content_type(media_type: &str) -> bool {
    media_type.starts_with("image/")
        || media_type.starts_with("audio/")
        || media_type.starts_with("video/")
        || media_type == "application/pdf"
        || media_type == "application/zip"
        || media_type == "application/octet-stream"
}

fn starts_with_tag(bytes: &[u8], pos: usize, tag: &[u8]) -> bool {
    let end = pos + tag.len();
    if end > bytes.len() {
        return false;
    }
    if !bytes[pos..end].eq_ignore_ascii_case(tag) {
        return false;
    }
    match bytes.get(end) {
        None => true,
        Some(&b) => b.is_ascii_whitespace() || b == b'>' || b == b'/',
    }
}

fn strip_tag_blocks(html: &str, tag: &str) -> String {
    let bytes = html.as_bytes();
    let tag_bytes = tag.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' && starts_with_tag(bytes, i + 1, tag_bytes) {
            let mut j = i + 1 + tag_bytes.len();
            let mut found_end = None;
            while j + 2 + tag_bytes.len() <= bytes.len() {
                if bytes[j] == b'<'
                    && bytes[j + 1] == b'/'
                    && starts_with_tag(bytes, j + 2, tag_bytes)
                {
                    if let Some(gt) = bytes[j + 2 + tag_bytes.len()..].iter().position(|&b| b == b'>') {
                        found_end = Some(j + 2 + tag_bytes.len() + gt + 1);
                    } else {
                        found_end = Some(bytes.len());
                    }
                    break;
                }
                j += 1;
            }
            match found_end {
                Some(next) => {
                    out.push(' ');
                    i = next;
                }
                None => break,
            }
        } else {
            let ch = html[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn extract_title(html: &str) -> Option<String> {
    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' && starts_with_tag(bytes, i + 1, b"title") {
            let gt = bytes[i..].iter().position(|&b| b == b'>')?;
            let content_start = i + gt + 1;
            let mut j = content_start;
            while j + 7 <= bytes.len() {
                if bytes[j] == b'<' && bytes[j + 1] == b'/' && starts_with_tag(bytes, j + 2, b"title") {
                    let raw = &html[content_start..j];
                    let decoded = decode_html_entities(&strip_html_tags(raw));
                    let cleaned = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
                    return (!cleaned.is_empty()).then_some(cleaned);
                }
                j += 1;
            }
            return None;
        }
        i += 1;
    }
    None
}

fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                if in_tag {
                    in_tag = false;
                    out.push(' ');
                } else {
                    out.push(ch);
                }
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn decode_html_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let replacements = [
            ("&amp;", "&"),
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&quot;", "\""),
            ("&#39;", "'"),
            ("&apos;", "'"),
            ("&nbsp;", " "),
        ];
        if let Some((entity, val)) = replacements.iter().find(|(e, _)| tail.starts_with(e)) {
            out.push_str(val);
            rest = &tail[entity.len()..];
        } else {
            out.push('&');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

fn extract_html_text(html: &str) -> (Option<String>, String) {
    let title = extract_title(html);
    let mut cleaned = strip_tag_blocks(html, "script");
    cleaned = strip_tag_blocks(&cleaned, "style");
    cleaned = strip_tag_blocks(&cleaned, "noscript");
    cleaned = strip_tag_blocks(&cleaned, "svg");
    let stripped = strip_html_tags(&cleaned);
    let decoded = decode_html_entities(&stripped);
    let collapsed = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    (title, collapsed)
}

/// Fetches a web page with SSRF protection (public-IP DNS check + pinned resolution + manual redirects).
pub async fn fetch_web_page(url: &str, allow_private: bool) -> Result<String, String> {
    let mut current = reqwest::Url::parse(url.trim()).map_err(|_| "invalid URL".to_string())?;

    for hop in 0..=MAX_FETCH_REDIRECTS {
        let scheme = current.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(format!("unsupported URL scheme: {scheme}"));
        }
        if !current.username().is_empty() || current.password().is_some() {
            return Err("URLs with embedded credentials are not allowed".into());
        }
        let host = current.host_str().ok_or_else(|| "URL has no host".to_string())?;
        let host_clean = host.trim_start_matches('[').trim_end_matches(']');
        let port = current
            .port_or_known_default()
            .ok_or_else(|| "URL has no port".to_string())?;

        let (domain_to_pin, addrs): (Option<String>, Vec<SocketAddr>) =
            if let Ok(ip) = host_clean.parse::<IpAddr>() {
                if !allow_private && !is_public_ip(ip) {
                    return Err(format!("blocked non-public IP address: {ip}"));
                }
                (None, vec![SocketAddr::new(ip, port)])
            } else {
                let resolved: Vec<SocketAddr> = tokio::net::lookup_host((host_clean, port))
                    .await
                    .map_err(|e| format!("DNS resolution failed: {e}"))?
                    .collect();
                if resolved.is_empty() {
                    return Err("DNS resolution returned no addresses".into());
                }
                if !allow_private {
                    for addr in &resolved {
                        if !is_public_ip(addr.ip()) {
                            return Err(format!("blocked non-public IP address: {}", addr.ip()));
                        }
                    }
                }
                (Some(host_clean.to_string()), resolved)
            };

        let mut builder = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("Duragon/", env!("CARGO_PKG_VERSION")));
        if let Some(domain) = domain_to_pin.as_deref() {
            builder = builder.resolve_to_addrs(domain, &addrs);
        }
        let client = builder
            .build()
            .map_err(|e| format!("failed to build HTTP client: {e}"))?;

        let mut resp = client
            .get(current.clone())
            .header(
                reqwest::header::ACCEPT,
                "text/html, text/plain, application/json;q=0.9, */*;q=0.1",
            )
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e.without_url()))?;

        if resp.status().is_redirection() {
            if hop == MAX_FETCH_REDIRECTS {
                return Err("too many redirects".into());
            }
            let location = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| "redirect missing Location header".to_string())?;
            current = current
                .join(location)
                .map_err(|_| "invalid redirect URL".to_string())?;
            continue;
        }

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status().as_u16()));
        }

        let media_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .and_then(|ct| ct.split(';').next())
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if is_blocked_content_type(&media_type) {
            return Err(format!("unsupported content type: {media_type}"));
        }

        let mut buf: Vec<u8> = Vec::new();
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| format!("failed reading response body: {}", e.without_url()))?
        {
            let remaining = MAX_FETCH_BYTES.saturating_sub(buf.len());
            if remaining == 0 {
                break;
            }
            let take = chunk.len().min(remaining);
            buf.extend_from_slice(&chunk[..take]);
            if buf.len() >= MAX_FETCH_BYTES {
                break;
            }
        }

        let raw = String::from_utf8_lossy(&buf);
        let is_html = media_type == "text/html"
            || media_type == "application/xhtml+xml"
            || (media_type.is_empty() && raw.trim_start().starts_with('<'));

        let body = if is_html {
            let (title, text) = extract_html_text(&raw);
            match title {
                Some(t) if !t.is_empty() => format!("Title: {t}\n\n{text}"),
                _ => text,
            }
        } else {
            raw.into_owned()
        };

        let mut out = format!("Source: {current}\n\n{body}");
        if out.chars().count() > MAX_FETCH_CHARS {
            let truncated: String = out.chars().take(MAX_FETCH_CHARS).collect();
            out = format!("{truncated}\n\n[Truncated]");
        }
        return Ok(out);
    }

    Err("too many redirects".into())
}

/// Runs the `web_fetch` tool call for an AI provider loop. Never panics or aborts the loop.
pub async fn run_web_fetch_tool(args: &Value) -> String {
    let url = args.get("url").and_then(Value::as_str).map(str::trim).unwrap_or("");
    if url.is_empty() {
        return "web_fetch failed: url is empty".into();
    }
    match fetch_web_page(url, false).await {
        Ok(text) => text,
        Err(err) => format!("web_fetch failed: {err}"),
    }
}
