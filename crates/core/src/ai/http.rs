//! Direct HTTP API providers: OpenAI-compatible (OpenAI, OpenRouter, DeepSeek, Z.AI/GLM, MiMo, custom),
//! Anthropic Messages API, and Google Gemini API.
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::LazyLock;

use super::notes_mcp;
use super::runner::{AiError, AiRequest};
use super::schema_util::{extract_json, gemini_schema, has_required, json_instruction, strict_schema, strip_nulls};
use crate::model::ThinkingLevel;

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .user_agent(concat!("duragon/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client")
});

const MAX_TOOL_ROUNDS: usize = 6;

fn fail<T>(msg: impl Into<String>) -> Result<T, AiError> {
    Err(AiError::Failed(msg.into()))
}

fn require_key(req: &AiRequest) -> Result<&str, AiError> {
    req.api_key
        .as_deref()
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .ok_or_else(|| {
            let env_hint = req
                .provider
                .env_var
                .as_deref()
                .map(|v| format!(" (or set {v})"))
                .unwrap_or_default();
            AiError::Failed(format!(
                "No API key configured for {}{env_hint}. Add it in Customize → Models.",
                req.provider.name
            ))
        })
}

/// Only a built-in provider may have no address, because its key belongs to the default host.
fn base_url<'a>(req: &'a AiRequest, default: &'a str) -> Result<&'a str, AiError> {
    let set = req.provider.base_url.as_deref().map(str::trim).filter(|u| !u.is_empty());
    match set {
        Some(url) => Ok(url.trim_end_matches('/')),
        None if req.provider.builtin => Ok(default),
        None => fail(format!("{} has no address. Add it in Customize → Models.", req.provider.name)),
    }
}

pub(super) fn run_note_tool(req: &AiRequest, name: &str, args: &Value) -> String {
    let (Some(db), Some(notes)) = (&req.db, &req.notes) else {
        return "Notes search is unavailable.".into();
    };
    let exclude = &notes.open_note_id;
    let res = match name {
        "search_notes" => {
            let q = args.get("query").and_then(Value::as_str).unwrap_or("");
            notes_mcp::search(db, exclude, q)
        }
        "read_note" => {
            let id = args.get("id").and_then(Value::as_str).unwrap_or("");
            notes_mcp::read(db, exclude, id)
        }
        other => Err(format!("unknown tool: {other}")),
    };
    match res {
        Ok(text) | Err(text) => text,
    }
}

pub(super) fn note_tools() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "search_notes",
                "description": "Full-text search over the writer's other notes (not the one open now). Returns up to 8 matches, best first, each with its id, title, last-edited date and an excerpt.",
                "parameters": {
                    "type": "object",
                    "properties": { "query": { "type": "string", "description": "A few words to look for" } },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_note",
                "description": "Reads one of the writer's other notes in full, by the id search_notes returned.",
                "parameters": {
                    "type": "object",
                    "properties": { "id": { "type": "string", "description": "The note id from search_notes" } },
                    "required": ["id"]
                }
            }
        }
    ])
}

/// OpenAI-compatible Chat Completions (OpenAI, OpenRouter, DeepSeek, Z.AI/GLM, MiMo, custom).
pub async fn run_openai(req: &AiRequest) -> Result<Value, AiError> {
    let key = require_key(req)?;
    let base = base_url(req, "https://api.openai.com/v1")?;
    let url = format!("{base}/chat/completions");
    let is_openrouter = base.contains("openrouter.ai");

    let mut model = req.model.trim().to_string();
    if is_openrouter && req.web && !model.ends_with(":online") {
        model.push_str(":online");
    }

    let system = format!("{}{}", req.system_prompt, json_instruction(&req.schema));
    let mut messages = vec![
        json!({ "role": "system", "content": system }),
        json!({ "role": "user", "content": req.payload }),
    ];

    let strict = strict_schema(&req.schema);
    let mut use_json_schema = true;
    let mut send_reasoning = req.thinking != ThinkingLevel::Default;

    for _round in 0..MAX_TOOL_ROUNDS {
        let response_format = if use_json_schema {
            json!({
                "type": "json_schema",
                "json_schema": { "name": "duragon_output", "strict": true, "schema": strict }
            })
        } else {
            json!({ "type": "json_object" })
        };

        let mut body = json!({
            "model": model,
            "messages": messages,
            "response_format": response_format,
        });
        if send_reasoning {
            let effort = match req.thinking {
                ThinkingLevel::Low => "low",
                ThinkingLevel::Medium => "medium",
                ThinkingLevel::High => "high",
                ThinkingLevel::XHigh => "high",
                ThinkingLevel::Default => unreachable!(),
            };
            body["reasoning_effort"] = json!(effort);
        }
        if req.notes.is_some() && req.db.is_some() {
            body["tools"] = note_tools();
        }

        let mut builder = CLIENT
            .post(&url)
            .bearer_auth(key)
            .timeout(req.timeout)
            .json(&body);
        if is_openrouter {
            builder = builder
                .header("HTTP-Referer", "https://duragon.app")
                .header("X-Title", "Duragon");
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| AiError::Failed(format!("{} request failed: {e}", req.provider.name)))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| AiError::Failed(format!("{} read failed: {e}", req.provider.name)))?;

        if !status.is_success() {
            // Fall back cleanly if an OpenAI-compatible endpoint rejects `reasoning_effort` or `json_schema`.
            if status.as_u16() == 400 && send_reasoning {
                send_reasoning = false;
                continue;
            }
            if status.as_u16() == 400 && use_json_schema {
                use_json_schema = false;
                continue;
            }
            let preview: String = text.chars().take(350).collect();
            return fail(format!("{} HTTP {}: {preview}", req.provider.name, status.as_u16()));
        }

        let parsed: Value = serde_json::from_str(&text)
            .map_err(|e| AiError::Failed(format!("{} invalid JSON response: {e}", req.provider.name)))?;
        let msg = &parsed["choices"][0]["message"];

        if let Some(calls) = msg.get("tool_calls").and_then(Value::as_array) {
            if !calls.is_empty() {
                messages.push(msg.clone());
                for call in calls {
                    let id = call.get("id").and_then(Value::as_str).unwrap_or("");
                    let name = call["function"].get("name").and_then(Value::as_str).unwrap_or("");
                    let args_str = call["function"].get("arguments").and_then(Value::as_str).unwrap_or("{}");
                    let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));
                    let out = run_note_tool(req, name, &args);
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": id,
                        "content": out,
                    }));
                }
                continue;
            }
        }

        let content = msg.get("content").and_then(Value::as_str).unwrap_or("");
        return extract_json(content).map_err(|e| AiError::Failed(format!("{}: {e}", req.provider.name)));
    }

    fail(format!("{} exceeded maximum tool-call rounds", req.provider.name))
}

/// How a request asks Claude to think. Which one a model takes depends on its age.
#[derive(Clone, Copy)]
enum Thinking {
    ModelsOwn,
    Effort,
    Budget,
}

fn budget_of(level: ThinkingLevel) -> u32 {
    match level {
        ThinkingLevel::Default | ThinkingLevel::Low => 1024,
        ThinkingLevel::Medium => 4096,
        ThinkingLevel::High => 10_000,
        ThinkingLevel::XHigh => 24_000,
    }
}

/// Anthropic Messages API (`POST /v1/messages`). `respond` can't be forced, because newer models refuse that.
pub async fn run_anthropic(req: &AiRequest) -> Result<Value, AiError> {
    let key = require_key(req)?;
    let base = base_url(req, "https://api.anthropic.com")?;
    let url = format!("{base}/v1/messages");

    let mut tools = vec![json!({
        "name": "respond",
        "description": "Return your final structured response matching the required schema.",
        "input_schema": req.schema
    })];
    let has_notes = req.notes.is_some() && req.db.is_some();
    if has_notes {
        tools.push(json!({
            "name": "search_notes",
            "description": "Full-text search over the writer's other notes (not the one open now).",
            "input_schema": {
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            }
        }));
        tools.push(json!({
            "name": "read_note",
            "description": "Reads one of the writer's other notes in full by its id.",
            "input_schema": {
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }
        }));
    }
    if req.web {
        tools.push(json!({
            "type": "web_search_20250305",
            "name": "web_search",
            "max_uses": 5
        }));
    }

    let system = format!(
        "{}\n\nWhen you have your final answer, call the `respond` tool with it. Do not write it as text.",
        req.system_prompt
    );
    let mut messages = vec![json!({ "role": "user", "content": req.payload })];
    let mut thinking = if req.thinking == ThinkingLevel::Default { Thinking::ModelsOwn } else { Thinking::Effort };
    let mut rounds = 0;

    while rounds < MAX_TOOL_ROUNDS {
        let mut body = json!({
            "model": req.model.trim(),
            "max_tokens": 16_000,
            "system": system,
            "messages": messages,
            "tools": tools,
        });
        match thinking {
            Thinking::ModelsOwn => {}
            Thinking::Effort => {
                body["max_tokens"] = json!(32_000);
                body["thinking"] = json!({ "type": "adaptive" });
                body["output_config"] = json!({ "effort": req.thinking.as_str() });
            }
            Thinking::Budget => {
                body["max_tokens"] = json!(32_000);
                body["thinking"] = json!({ "type": "enabled", "budget_tokens": budget_of(req.thinking) });
            }
        }

        let resp = CLIENT
            .post(&url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .timeout(req.timeout)
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Failed(format!("Anthropic request failed: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| AiError::Failed(format!("Anthropic read failed: {e}")))?;
        if !status.is_success() {
            // Newer models take an effort, older ones a budget, and the oldest neither.
            if status.as_u16() == 400 && (text.contains("thinking") || text.contains("effort")) {
                match thinking {
                    Thinking::Effort => {
                        thinking = Thinking::Budget;
                        continue;
                    }
                    Thinking::Budget => {
                        thinking = Thinking::ModelsOwn;
                        continue;
                    }
                    Thinking::ModelsOwn => {}
                }
            }
            let preview: String = text.chars().take(350).collect();
            return fail(format!("Anthropic HTTP {}: {preview}", status.as_u16()));
        }
        rounds += 1;

        let parsed: Value = serde_json::from_str(&text)
            .map_err(|e| AiError::Failed(format!("Anthropic invalid JSON: {e}")))?;
        let content = parsed
            .get("content")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        // Check for `respond` tool call first.
        for block in &content {
            if block.get("type").and_then(Value::as_str) == Some("tool_use")
                && block.get("name").and_then(Value::as_str) == Some("respond")
            {
                return Ok(strip_nulls(block.get("input").cloned().unwrap_or(json!({}))));
            }
        }

        // Handle notes tool calls if any.
        let mut tool_results = Vec::new();
        for block in &content {
            if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                let name = block.get("name").and_then(Value::as_str).unwrap_or("");
                if name == "search_notes" || name == "read_note" {
                    let id = block.get("id").and_then(Value::as_str).unwrap_or("");
                    let input = block.get("input").unwrap_or(&Value::Null);
                    let out = run_note_tool(req, name, input);
                    tool_results.push(json!({
                        "type": "tool_result",
                        "tool_use_id": id,
                        "content": out
                    }));
                }
            }
        }
        if !tool_results.is_empty() {
            messages.push(json!({ "role": "assistant", "content": content }));
            messages.push(json!({ "role": "user", "content": tool_results }));
            continue;
        }

        match parsed["stop_reason"].as_str() {
            Some("refusal") => return fail("Claude declined to answer this"),
            Some("max_tokens") => return fail("Claude ran out of room before it finished"),
            Some("pause_turn") => {
                messages.push(json!({ "role": "assistant", "content": content }));
                continue;
            }
            _ => {}
        }
        let said: Vec<&str> = content.iter().filter_map(|b| b["text"].as_str()).collect();
        match extract_json(&said.join("\n")) {
            Ok(answer) if has_required(&answer, &req.schema) => return Ok(answer),
            _ if content.is_empty() => return fail("Claude answered with nothing"),
            _ => {
                messages.push(json!({ "role": "assistant", "content": content }));
                messages.push(json!({ "role": "user", "content": "Call the `respond` tool with that answer." }));
            }
        }
    }

    fail("Claude did not give its answer in the form Duragon needs")
}

/// Google Gemini API (`POST /models/{model}:generateContent`).
pub async fn run_gemini(req: &AiRequest) -> Result<Value, AiError> {
    let key = require_key(req)?;
    let base = base_url(req, "https://generativelanguage.googleapis.com/v1beta")?;
    let model = req.model.trim();
    let url = format!("{base}/models/{model}:generateContent");

    let has_notes = req.notes.is_some() && req.db.is_some();
    let mut tools: Vec<Value> = Vec::new();
    if has_notes {
        tools.push(json!({
            "functionDeclarations": [
                {
                    "name": "search_notes",
                    "description": "Full-text search over the writer's other notes (not the one open now).",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": { "query": { "type": "STRING" } },
                        "required": ["query"]
                    }
                },
                {
                    "name": "read_note",
                    "description": "Reads one of the writer's other notes in full by its id.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": { "id": { "type": "STRING" } },
                        "required": ["id"]
                    }
                }
            ]
        }));
    }
    if req.web {
        tools.push(json!({ "googleSearch": {} }));
    }

    // Gemini rejects combining `responseSchema` with `googleSearch` on some models;
    // when tools are active, rely on prompt JSON schema + extraction, otherwise use native `responseSchema`.
    let use_native_schema = tools.is_empty();
    let system = if use_native_schema {
        req.system_prompt.clone()
    } else {
        format!("{}{}", req.system_prompt, json_instruction(&req.schema))
    };

    let mut contents = vec![json!({
        "role": "user",
        "parts": [{ "text": req.payload }]
    })];

    for _round in 0..MAX_TOOL_ROUNDS {
        let mut gen_cfg = json!({});
        if use_native_schema {
            gen_cfg["responseMimeType"] = json!("application/json");
            gen_cfg["responseSchema"] = gemini_schema(&req.schema);
        }
        if req.thinking != ThinkingLevel::Default {
            let budget = match req.thinking {
                ThinkingLevel::Low => 1024,
                ThinkingLevel::Medium => 4096,
                ThinkingLevel::High => 10000,
                ThinkingLevel::XHigh => 24576,
                ThinkingLevel::Default => unreachable!(),
            };
            gen_cfg["thinkingConfig"] = json!({ "thinkingBudget": budget });
        }
        let mut body = json!({
            "systemInstruction": { "parts": [{ "text": system }] },
            "contents": contents,
            "generationConfig": gen_cfg,
        });
        if !tools.is_empty() {
            body["tools"] = Value::Array(tools.clone());
        }

        let resp = CLIENT
            .post(&url)
            .header("x-goog-api-key", key)
            .timeout(req.timeout)
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Failed(format!("Gemini request failed: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| AiError::Failed(format!("Gemini read failed: {e}")))?;
        if !status.is_success() {
            let preview: String = text.chars().take(350).collect();
            return fail(format!("Gemini HTTP {}: {preview}", status.as_u16()));
        }

        let parsed: Value = serde_json::from_str(&text)
            .map_err(|e| AiError::Failed(format!("Gemini invalid JSON: {e}")))?;
        let parts = parsed["candidates"][0]["content"]["parts"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        // Check for function calls.
        let mut fn_responses = Vec::new();
        for part in &parts {
            if let Some(fc) = part.get("functionCall") {
                let name = fc.get("name").and_then(Value::as_str).unwrap_or("");
                let args = fc.get("args").unwrap_or(&Value::Null);
                let out = run_note_tool(req, name, args);
                fn_responses.push(json!({
                    "functionResponse": {
                        "name": name,
                        "response": { "result": out }
                    }
                }));
            }
        }
        if !fn_responses.is_empty() {
            contents.push(json!({ "role": "model", "parts": parts }));
            contents.push(json!({ "role": "user", "parts": fn_responses }));
            continue;
        }

        let combined: String = parts
            .iter()
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
        return extract_json(&combined).map_err(|e| AiError::Failed(format!("Gemini: {e}")));
    }

    fail("Gemini exceeded maximum tool-call rounds")
}
