//! JSON Schema normalization and structured-output extraction shared across AI providers.
use serde_json::{json, Map, Value};

/// Converts a Duragon task schema into OpenAI/Codex strict JSON Schema format:
/// every property in `properties` is listed in `required`, and properties that were
/// previously optional become nullable (`["<type>", "null"]`).
pub fn strict_schema(schema: &Value) -> Value {
    let mut out = schema.clone();
    enforce_strict(&mut out);
    out
}

fn enforce_strict(node: &mut Value) {
    let Some(obj) = node.as_object_mut() else { return };
    if obj.get("type").and_then(Value::as_str) == Some("object") {
        obj.insert("additionalProperties".into(), json!(false));
        let existing_req: Vec<String> = obj
            .get("required")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        if let Some(props) = obj.get_mut("properties").and_then(Value::as_object_mut) {
            let all_keys: Vec<String> = props.keys().cloned().collect();
            for (k, prop) in props.iter_mut() {
                enforce_strict(prop);
                if !existing_req.contains(k) {
                    make_nullable(prop);
                }
            }
            obj.insert("required".into(), json!(all_keys));
        }
    }
    if let Some(items) = obj.get_mut("items") {
        enforce_strict(items);
    }
}

fn make_nullable(prop: &mut Value) {
    let Some(obj) = prop.as_object_mut() else { return };
    match obj.get("type").cloned() {
        Some(Value::String(t)) if t != "null" => {
            obj.insert("type".into(), json!([t, "null"]));
        }
        Some(Value::Array(mut arr)) if !arr.iter().any(|v| v.as_str() == Some("null")) => {
            arr.push(json!("null"));
            obj.insert("type".into(), Value::Array(arr));
        }
        _ => {}
    }
}

/// Converts a Duragon task schema into Google Gemini's `responseSchema` subset
/// (which rejects `additionalProperties`).
pub fn gemini_schema(schema: &Value) -> Value {
    let mut out = schema.clone();
    strip_additional_properties(&mut out);
    out
}

fn strip_additional_properties(node: &mut Value) {
    let Some(obj) = node.as_object_mut() else { return };
    obj.remove("additionalProperties");
    if let Some(props) = obj.get_mut("properties").and_then(Value::as_object_mut) {
        for v in props.values_mut() {
            strip_additional_properties(v);
        }
    }
    if let Some(items) = obj.get_mut("items") {
        strip_additional_properties(items);
    }
}

/// Removes top-level `null` values from a JSON object so `#[serde(default)]` fields
/// (like `drawing: bool`, `sources: Vec<_>`, `symbols: Vec<_>`) deserialize cleanly
/// after strict-schema providers emit explicit `null` for optional keys.
pub fn strip_nulls(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let cleaned: Map<String, Value> = map
                .into_iter()
                .filter(|(_, val)| !val.is_null())
                .collect();
            Value::Object(cleaned)
        }
        other => other,
    }
}

/// Extracts a JSON object from raw model text (handles plain JSON, ```json fences,
/// or leading/trailing prose around a `{ ... }` object).
pub fn extract_json(raw: &str) -> Result<Value, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty response".into());
    }
    if let Ok(v @ Value::Object(_)) = serde_json::from_str::<Value>(trimmed) {
        return Ok(strip_nulls(v));
    }
    // Check fenced code blocks (prefer the last valid JSON object block).
    let mut found: Option<Value> = None;
    let mut rest = trimmed;
    while let Some(start) = rest.find("```") {
        rest = &rest[start + 3..];
        let Some(nl) = rest.find('\n') else { break };
        let after_lang = &rest[nl + 1..];
        let Some(end) = after_lang.find("```") else { break };
        let body = after_lang[..end].trim();
        if let Ok(v @ Value::Object(_)) = serde_json::from_str::<Value>(body) {
            found = Some(v);
        }
        rest = &after_lang[end + 3..];
    }
    if let Some(v) = found {
        return Ok(strip_nulls(v));
    }
    // Fallback: slice from first '{' to last '}'.
    if let (Some(a), Some(b)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if b > a {
            if let Ok(v @ Value::Object(_)) = serde_json::from_str::<Value>(&trimmed[a..=b]) {
                return Ok(strip_nulls(v));
            }
        }
    }
    let preview: String = trimmed.chars().take(200).collect();
    Err(format!("could not parse JSON object from response: {preview}"))
}

/// Appends explicit JSON schema output instructions to a system prompt for providers
/// that rely on prompt-guided JSON output (e.g., ACP agents or `json_object` fallback).
pub fn json_instruction(schema: &Value) -> String {
    format!(
        "\n\nOUTPUT FORMAT: Respond with ONLY a single valid JSON object matching this JSON Schema (no markdown fences, no commentary outside the JSON):\n{schema}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_makes_optional_nullable_and_required() {
        let s = json!({
            "type": "object",
            "properties": {
                "said": { "type": "boolean" },
                "message": { "type": "string" }
            },
            "required": ["said"],
            "additionalProperties": false
        });
        let st = strict_schema(&s);
        assert_eq!(st["properties"]["said"]["type"], json!("boolean"));
        assert_eq!(st["properties"]["message"]["type"], json!(["string", "null"]));
        let req = st["required"].as_array().unwrap();
        assert_eq!(req.len(), 2);
    }

    #[test]
    fn extract_handles_fences_and_nulls() {
        let raw = "Here you go:\n```json\n{\"said\": true, \"message\": \"hi\", \"drawing\": null}\n```";
        let v = extract_json(raw).unwrap();
        assert_eq!(v["said"], json!(true));
        assert_eq!(v["message"], json!("hi"));
        assert!(v.get("drawing").is_none());
    }
}
