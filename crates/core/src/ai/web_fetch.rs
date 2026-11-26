//! SSRF-protected web page fetcher and single-pass HTML text extractor.
use reqwest::Client;
use serde_json::Value;
use std::net::{IpAddr, SocketAddr};
use std::sync::LazyLock;
use std::time::Duration;

const MAX_FETCH_BYTES: usize = 2 * 1024 * 1024;
const MAX_FETCH_CHARS: usize = 24_000;
const MAX_FETCH_REDIRECTS: usize = 3;

static FETCH_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    fetch_client_builder()
        .build()
        .expect("reqwest web_fetch client")
});

fn fetch_client_builder() -> reqwest::ClientBuilder {
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("Duragon/", env!("CARGO_PKG_VERSION")))
}

/// Returns `true` only if `ip` is a globally routable public address.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, c, _] = v4.octets();
            !(a == 0 // 0.0.0.0/8
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
                || a >= 224) // 224.0.0.0/4 (multicast) & 240.0.0.0/4 (reserved/broadcast)
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            let s = v6.segments();
            if (s[0] & 0xe000) != 0x2000 {
                return false;
            }
            !((s[0] == 0x2001 && (s[1] & 0xfe00) == 0x0000) // 2001::/23 (Teredo, ORCHID, benchmarking, IETF)
                || (s[0] == 0x2001 && s[1] == 0x0db8) // 2001:db8::/32 (documentation)
                || s[0] == 0x2002) // 2002::/16 (6to4)
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

const SKIP_TAGS: [&[u8]; 4] = [b"script", b"style", b"noscript", b"svg"];

const ENTITIES: [(&str, char); 7] = [
    ("&amp;", '&'),
    ("&lt;", '<'),
    ("&gt;", '>'),
    ("&quot;", '"'),
    ("&#39;", '\''),
    ("&apos;", '\''),
    ("&nbsp;", ' '),
];

fn starts_with_tag(bytes: &[u8], pos: usize, tag: &[u8]) -> bool {
    let end = pos + tag.len();
    if end > bytes.len() || !bytes[pos..end].eq_ignore_ascii_case(tag) {
        return false;
    }
    match bytes.get(end) {
        None => true,
        Some(&b) => b.is_ascii_whitespace() || b == b'>' || b == b'/',
    }
}

struct CollapsedText {
    out: String,
    pending_space: bool,
}

impl CollapsedText {
    fn with_capacity(cap: usize) -> Self {
        Self { out: String::with_capacity(cap), pending_space: false }
    }

    fn mark_space(&mut self) {
        if !self.out.is_empty() {
            self.pending_space = true;
        }
    }

    fn push_char(&mut self, ch: char) {
        if ch.is_whitespace() {
            self.mark_space();
        } else {
            if self.pending_space {
                self.out.push(' ');
                self.pending_space = false;
            }
            self.out.push(ch);
        }
    }
}

/// Extracts `(title, body_text)` in a single pass, stripping `script`/`style`/`noscript`/`svg`
/// blocks, HTML tags, decoding common entities, and collapsing whitespace.
fn extract_html_text(html: &str) -> (Option<String>, String) {
    let bytes = html.as_bytes();
    let mut body = CollapsedText::with_capacity(html.len() / 2);
    let mut title_buf = CollapsedText::with_capacity(64);
    let mut title: Option<String> = None;
    let mut in_title = false;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'<' {
            if in_title && bytes.get(i + 1) == Some(&b'/') && starts_with_tag(bytes, i + 2, b"title") {
                in_title = false;
                if title.is_none() && !title_buf.out.is_empty() {
                    title = Some(std::mem::take(&mut title_buf.out));
                }
                i = bytes[i..].iter().position(|&b| b == b'>').map_or(bytes.len(), |gt| i + gt + 1);
                body.mark_space();
                continue;
            }

            if !in_title && title.is_none() && starts_with_tag(bytes, i + 1, b"title") {
                if let Some(gt) = bytes[i..].iter().position(|&b| b == b'>') {
                    let self_closing = gt > 0 && bytes[i + gt - 1] == b'/';
                    i += gt + 1;
                    if !self_closing {
                        in_title = true;
                        title_buf.out.clear();
                        title_buf.pending_space = false;
                    }
                    continue;
                }
                break;
            }

            if let Some(&skip_tag) = SKIP_TAGS.iter().find(|&&tag| starts_with_tag(bytes, i + 1, tag)) {
                if let Some(gt) = bytes[i..].iter().position(|&b| b == b'>') {
                    let self_closing = gt > 0 && bytes[i + gt - 1] == b'/';
                    let mut j = i + gt + 1;
                    if self_closing {
                        i = j;
                        body.mark_space();
                        continue;
                    }
                    let mut found_end = None;
                    while j + 2 + skip_tag.len() <= bytes.len() {
                        if bytes[j] == b'<'
                            && bytes[j + 1] == b'/'
                            && starts_with_tag(bytes, j + 2, skip_tag)
                        {
                            let after = j + 2 + skip_tag.len();
                            found_end = Some(
                                bytes[after..]
                                    .iter()
                                    .position(|&b| b == b'>')
                                    .map_or(bytes.len(), |end_gt| after + end_gt + 1),
                            );
                            break;
                        }
                        j += 1;
                    }
                    match found_end {
                        Some(next) => {
                            i = next;
                            body.mark_space();
                            continue;
                        }
                        None => break,
                    }
                }
                break;
            }

            i = bytes[i..].iter().position(|&b| b == b'>').map_or(bytes.len(), |gt| i + gt + 1);
            if in_title {
                title_buf.mark_space();
            } else {
                body.mark_space();
            }
            continue;
        }

        let (ch, step) = if bytes[i] == b'&' {
            let tail = &html[i..];
            if let Some(&(entity, decoded)) = ENTITIES.iter().find(|(e, _)| tail.starts_with(e)) {
                (decoded, entity.len())
            } else {
                ('&', 1)
            }
        } else {
            let ch = html[i..].chars().next().unwrap();
            (ch, ch.len_utf8())
        };

        if in_title {
            title_buf.push_char(ch);
        } else {
            body.push_char(ch);
        }
        i += step;
    }

    (title, body.out)
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

        let pinned_client;
        let client = if let Some(domain) = domain_to_pin.as_deref() {
            pinned_client = fetch_client_builder()
                .resolve_to_addrs(domain, &addrs)
                .build()
                .map_err(|e| format!("failed to build HTTP client: {e}"))?;
            &pinned_client
        } else {
            &*FETCH_CLIENT
        };

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

/// Runs the `web_fetch` tool call for an AI provider loop or MCP server.
pub async fn run_web_fetch_tool(args: &Value) -> Result<String, String> {
    let url = args.get("url").and_then(Value::as_str).map(str::trim).unwrap_or("");
    if url.is_empty() {
        return Err("web_fetch failed: url is empty".into());
    }
    fetch_web_page(url, false)
        .await
        .map_err(|err| format!("web_fetch failed: {err}"))
}
