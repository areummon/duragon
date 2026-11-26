//! Model servers the writer runs: Ollama through its own API, and llama.cpp, vLLM and the like through OpenAI's.
use reqwest::{Client, RequestBuilder};
use serde_json::{json, Value};
use std::sync::LazyLock;
use std::time::Duration;

use super::budget::tokens;
use super::http::{openai_tools, run_tool};
use super::runner::{AiError, AiRequest};
use super::schema_util::{extract_json, has_required, json_instruction, strict_schema};
use crate::credentials::{self, CredentialsStore};
use crate::db::Db;
use crate::model::{ProviderConfig, ProviderKind, ThinkingLevel};

/// Never through a proxy, which can't reach this computer and would see the note.
static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .connect_timeout(Duration::from_secs(3))
        .build()
        .expect("reqwest client")
});

/// Ollama reads 4096 tokens unless told, and drops the rest without a word. More costs memory.
const OLLAMA_WINDOW: usize = 16_384;
/// Room for the answer, and for the thinking before it.
const ANSWER: usize = 4_096;
/// Below this the note is too short a piece of itself.
const LEAST_PAYLOAD: usize = 512;
const MAX_TOOL_ROUNDS: usize = 6;
const MAX_TOOL_CHARS: usize = 6_000;
const LIST_TIMEOUT: Duration = Duration::from_secs(5);

enum Api {
    Ollama(String),
    OpenAi(String),
}

struct Model {
    name: String,
    window: Option<usize>,
}

struct Reply {
    message: Value,
    calls: Vec<(Value, String, Value)>,
}

/// What a request still asks of the server. A server that refuses one is asked again without it.
struct Asks {
    notes: bool,
    web_search: bool,
    thinking: bool,
    schema: bool,
}

impl Asks {
    fn has_tools(&self) -> bool {
        self.notes || self.web_search
    }
}

fn fail<T>(msg: impl Into<String>) -> Result<T, AiError> {
    Err(AiError::Failed(msg.into()))
}

impl Api {
    fn of(provider: &ProviderConfig) -> Result<Self, AiError> {
        let Some(raw) = provider.base_url.as_deref().map(str::trim).filter(|u| !u.is_empty()) else {
            return fail(format!("{} has no address. Add it in Customize → Providers.", provider.name));
        };
        let url = credentials::validate_base_url(raw)
            .map_err(|reason| AiError::Failed(format!("{}: {reason}", provider.name)))?;
        let root = url.strip_suffix("/v1").unwrap_or(&url);
        Ok(match provider.kind {
            ProviderKind::Ollama => Api::Ollama(root.into()),
            _ if root.split("://").nth(1).is_some_and(|rest| rest.contains('/')) => Api::OpenAi(url),
            _ => Api::OpenAi(format!("{root}/v1")),
        })
    }

    fn list(&self) -> RequestBuilder {
        match self {
            Api::Ollama(root) => CLIENT.get(format!("{root}/api/tags")),
            Api::OpenAi(base) => CLIENT.get(format!("{base}/models")),
        }
    }

    fn models(&self, listed: &Value) -> Vec<Model> {
        let window = |v: &Value| v.as_u64().map(|n| n as usize);
        let named = |list: &Value, key: &str, window_of: &dyn Fn(&Value) -> Option<usize>| -> Vec<Model> {
            let all = list.as_array().into_iter().flatten();
            all.filter_map(|m| Some(Model { name: m[key].as_str()?.to_string(), window: window_of(m) })).collect()
        };
        match self {
            Api::Ollama(_) => {
                let writes = |m: &&Value| m["capabilities"].as_array().is_none_or(|c| c.iter().any(|c| c == "completion"));
                let list: Vec<Value> = listed["models"].as_array().into_iter().flatten().filter(writes).cloned().collect();
                named(&json!(list), "name", &|m| Some(window(&m["details"]["context_length"]).unwrap_or(OLLAMA_WINDOW).min(OLLAMA_WINDOW)))
            }
            Api::OpenAi(_) => named(&listed["data"], "id", &|m| window(&m["meta"]["n_ctx"]).or_else(|| window(&m["max_model_len"]))),
        }
    }

    fn chat(&self, req: &AiRequest, messages: &[Value], asks: &Asks, window: usize) -> RequestBuilder {
        let mut body = json!({ "model": req.model.trim(), "messages": messages, "stream": false });
        let tools = openai_tools(asks.notes, asks.web_search);
        if !tools.is_empty() {
            body["tools"] = Value::Array(tools);
        }
        let schema = strict_schema(&req.schema);
        let (url, thinking) = match self {
            Api::Ollama(root) => {
                body["options"] = json!({ "num_ctx": window });
                body["truncate"] = json!(false);
                if asks.schema && !asks.has_tools() {
                    body["format"] = schema;
                }
                (format!("{root}/api/chat"), "think")
            }
            Api::OpenAi(base) => {
                if asks.schema && !asks.has_tools() {
                    body["response_format"] =
                        json!({ "type": "json_schema", "json_schema": { "name": "duragon_output", "strict": true, "schema": schema } });
                }
                (format!("{base}/chat/completions"), "reasoning_effort")
            }
        };
        if asks.thinking {
            body[thinking] = json!(match (self, req.thinking) {
                (Api::Ollama(_), ThinkingLevel::XHigh) => "max",
                (Api::OpenAi(_), ThinkingLevel::XHigh) => "high",
                (_, level) => level.as_str(),
            });
        }
        CLIENT.post(url).timeout(req.timeout).json(&body)
    }

    fn reply(&self, body: &Value) -> Reply {
        let message = match self {
            Api::Ollama(_) => &body["message"],
            Api::OpenAi(_) => &body["choices"][0]["message"],
        };
        let calls = message["tool_calls"].as_array().into_iter().flatten().map(|call| {
            let args = &call["function"]["arguments"];
            let args = args.as_str().map_or_else(|| args.clone(), |s| serde_json::from_str(s).unwrap_or_default());
            (call["id"].clone(), call["function"]["name"].as_str().unwrap_or_default().to_string(), args)
        });
        Reply { message: message.clone(), calls: calls.collect() }
    }
}

/// Sends a request and reads its JSON. A refusal is the status and what the server said.
async fn send(provider: &ProviderConfig, key: Option<&str>, request: RequestBuilder) -> Result<Result<Value, (u16, String)>, AiError> {
    let request = match key {
        Some(key) => request.bearer_auth(key),
        None => request,
    };
    let lost = |e: reqwest::Error| {
        let address = provider.base_url.as_deref().unwrap_or_default();
        AiError::Failed(if e.is_connect() { format!("Nothing answers at {address}") } else { format!("{}: {e}", provider.name) })
    };
    let response = request.send().await.map_err(lost)?;
    let status = response.status();
    let text = response.text().await.map_err(lost)?;
    let body: Option<Value> = serde_json::from_str(&text).ok();
    if status.is_success() {
        return body.map(Ok).ok_or_else(|| AiError::Failed(format!("{} did not answer with JSON", provider.name)));
    }
    let said = body.as_ref().map(|b| &b["error"]).and_then(|e| e.as_str().or_else(|| e["message"].as_str()));
    let said: String = said.unwrap_or(&text).chars().take(350).collect();
    Ok(Err((status.as_u16(), said)))
}

async fn list(provider: &ProviderConfig, key: Option<&str>) -> Result<Vec<Model>, AiError> {
    let api = Api::of(provider)?;
    match send(provider, key, api.list().timeout(LIST_TIMEOUT)).await? {
        Ok(listed) => Ok(api.models(&listed)),
        Err((status, said)) => fail(format!("{} answered {status}: {said}", provider.name)),
    }
}

/// The models a server the writer runs has now.
pub async fn local_models(db: &Db, store: &CredentialsStore, provider_id: &str) -> anyhow::Result<Vec<String>> {
    let provider = credentials::known_provider(db, provider_id)?;
    let key = store.resolve_for(&provider)?;
    let models = list(&provider, key.as_deref()).await.map_err(|e| match e {
        AiError::Failed(m) => anyhow::anyhow!(m),
        AiError::Cancelled => anyhow::anyhow!("cancelled"),
    })?;
    Ok(models.into_iter().map(|m| m.name).collect())
}

async fn window(req: &AiRequest) -> Result<Option<usize>, AiError> {
    let models = list(&req.provider, req.api_key.as_deref()).await?;
    let listed = models.into_iter().find(|m| m.name == req.model.trim()).and_then(|m| m.window);
    Ok(listed.or((req.provider.kind == ProviderKind::Ollama).then_some(OLLAMA_WINDOW)))
}

/// The payload tokens a call has room for, at most `budget`.
pub(super) async fn payload_budget(req: &AiRequest, budget: usize) -> Result<usize, AiError> {
    if !req.provider.kind.is_local_server() {
        return Ok(budget);
    }
    let Some(window) = window(req).await? else { return Ok(budget) };
    let instructions = tokens(&req.system_prompt) + tokens(&json_instruction(&req.schema));
    let room = window.saturating_sub(instructions + ANSWER.min(window / 4));
    // Other languages take more tokens than `tokens` counts.
    let room = room * 3 / 4;
    if room < LEAST_PAYLOAD {
        return fail(format!(
            "{} reads {window} tokens at once with {}, too few for Duragon’s instructions and a note. Start it with more.",
            req.provider.name, req.model
        ));
    }
    Ok(room.min(budget))
}

pub async fn run_local(req: &AiRequest) -> Result<Value, AiError> {
    let api = Api::of(&req.provider)?;
    let window = match &api {
        Api::Ollama(_) => window(req).await?.unwrap_or(OLLAMA_WINDOW),
        Api::OpenAi(_) => 0,
    };
    let mut asks = Asks {
        notes: req.notes.is_some() && req.db.is_some(),
        web_search: req.web_search.is_some(),
        thinking: req.thinking != ThinkingLevel::Default,
        schema: true,
    };
    let mut messages = vec![
        json!({ "role": "system", "content": format!("{}{}", req.system_prompt, json_instruction(&req.schema)) }),
        json!({ "role": "user", "content": req.payload }),
    ];
    let mut rounds = 0;
    loop {
        let body = match send(&req.provider, req.api_key.as_deref(), api.chat(req, &messages, &asks, window)).await? {
            Ok(body) => body,
            Err((400 | 500, _)) if asks.thinking => {
                asks.thinking = false;
                continue;
            }
            Err((400 | 500, _)) if asks.has_tools() => {
                asks.notes = false;
                asks.web_search = false;
                continue;
            }
            Err((400 | 500, said)) if asks.schema && !said.contains("context") => {
                asks.schema = false;
                continue;
            }
            Err((status, said)) => return fail(format!("{} answered {status}: {said}", req.provider.name)),
        };
        let Reply { message, calls } = api.reply(&body);
        if asks.has_tools() && !calls.is_empty() {
            messages.push(message);
            for (id, name, args) in calls {
                let found: String = run_tool(req, &name, &args).await.chars().take(MAX_TOOL_CHARS).collect();
                messages.push(json!({ "role": "tool", "tool_call_id": id, "tool_name": name, "content": found }));
            }
            rounds += 1;
            if rounds >= MAX_TOOL_ROUNDS {
                asks.notes = false;
                asks.web_search = false;
            }
            continue;
        }
        let answer = extract_json(message["content"].as_str().unwrap_or_default());
        // A schema keeps a model from calling tools, so it is only held to one once it has stopped.
        if asks.has_tools() && !answer.as_ref().is_ok_and(|a| has_required(a, &req.schema)) {
            asks.notes = false;
            asks.web_search = false;
            continue;
        }
        return answer.map_err(|e| AiError::Failed(format!("{}: {e}", req.provider.name)));
    }
}
