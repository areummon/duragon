# Providers & Web Search

Duragon separates **jobs** (what the AI is doing) from **providers** (where the model runs). You can mix and match providers per job in **Customize → Providers**.

## Jobs & Thinking Effort

Each job has its own **Model** selector and **Thinking** selector (`Auto`, `Low`, `Medium`, `High`, `Highest`):

| Job | Used for |
| :--- | :--- |
| **Reading** | Background margin questions, selection *Riddle this* / *Ask*, and thread replies |
| **Chat** | Side chat panel (`Ctrl+J`), inline answers (`Duragon, …`), and cross-note search |
| **Drawing** | Step-by-step SVG diagrams and *Visualize* |

## Supported AI Backends

| Kind | Examples | How Duragon runs it |
| :--- | :--- | :--- |
| **CLI runners** | Claude CLI (`claude`), Antigravity CLI (`agy` over ACP), Codex CLI (`codex`) | Spawns the local CLI binary in a clean environment (`quiet.rs` strips unrelated API keys) |
| **HTTP APIs** | Anthropic, OpenAI, Gemini, OpenRouter, custom OpenAI-compatible endpoints | Direct HTTPS calls from Rust (`crates/core/src/ai/http.rs`) with native tool calling |
| **Local servers** | Ollama, llama.cpp, LM Studio, any OpenAI-compatible local endpoint | Direct local HTTP calls (`crates/core/src/ai/local.rs`) with automatic server detection and model listing |

## Configurable Web Search & `web_fetch`

When **Web** is toggled on for a note (or enabled by default for new notes in **Customize → General**), Duragon can search the web and read pages using a configured search engine:

| Provider | Environment variable | Priority in `Auto` |
| :--- | :--- | :--- |
| **Tavily** | `TAVILY_API_KEY` | 1 |
| **Brave Search** | `BRAVE_API_KEY` | 2 |
| **Exa** | `EXA_API_KEY` | 3 |
| **Firecrawl** | `FIRECRAWL_API_KEY` | 4 |
| **Ollama** | `OLLAMA_API_KEY` | 5 |

Keys can be saved in **Customize → Providers → Web search** (stored in `credentials.json` with `0600` permissions) or provided via environment variables. Choosing **Automatic** picks the first configured engine in the table above; pinning a specific engine uses only that engine and never silently falls back if its key is missing.

### Native tools vs MCP vs Built-in fallback

| Backend | Web search provider configured | No web search provider configured |
| :--- | :--- | :--- |
| **HTTP APIs** | Native in-process `web_search` + `web_fetch` tools | No web tools |
| **Local models** | Native in-process `web_search` + `web_fetch` tools | No web tools |
| **Claude CLI & Antigravity (ACP)** | Duragon's MCP bridge (`--notes-mcp --web`) calling the exact same Rust `web_search` + `web_fetch` implementation | Falls back to each CLI's built-in web search (`WebSearch`/`WebFetch` on Claude, built-in web on Antigravity) |
| **Codex CLI** | Disabled when a custom search engine is pinned (respects user choice) | Uses Codex's built-in `web_search=live` |

### SSRF Protection on `web_fetch`

`crates/core/src/ai/web_fetch.rs` hardens page fetching before any connection is made:

- Only `http://` and `https://` URLs without embedded credentials are allowed.
- DNS is resolved up front and every returned IP is checked against a strict public-IP allowlist (blocking loopback, RFC1918 private networks, link-local / cloud metadata `169.254.169.254`, CGNAT, multicast, reserved ranges, IPv4-mapped/compatible IPv6, Teredo, ULA, and non-Global Unicast IPv6).
- Resolved IPs are pinned via `reqwest::ClientBuilder::resolve_to_addrs`, `.no_proxy()` prevents proxy-based DNS rebinding, redirects are followed manually (max 3 hops) with every hop re-validated, response bodies are capped at 2 MiB, and extracted text is truncated at 24k characters.

## Privacy & Credentials

- API keys are stored locally in `$XDG_DATA_HOME/dev.duragon.app/credentials.json` (written atomically with `0600` permissions).
- `Veil` (`crates/core/src/ai/veil.rs`) masks emails, known key prefixes (`sk-`, `tvly-`, `fc-`, `ghp_`, `AIza`, etc.), and active provider/search keys before outbound search queries or AI payloads leave the machine.
- Custom provider base URLs are validated and normalized before saving.
