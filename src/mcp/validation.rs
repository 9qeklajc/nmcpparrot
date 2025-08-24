use serde_json::{Map, Value};

/// Sanitizes JSON parameters by cleaning malformed JSON and removing trailing characters
#[allow(dead_code)] // Future use for JSON validation
pub fn sanitize_json_parameters(params: &str) -> Result<String, String> {
    if params.trim().is_empty() {
        return Ok("{}".to_string());
    }

    // First, try to extract just the first complete JSON object
    if let Some(clean_json) = extract_first_json_object(params) {
        match serde_json::from_str::<Value>(&clean_json) {
            Ok(value) => {
                let sanitized = sanitize_value(value);
                return Ok(serde_json::to_string(&sanitized).unwrap_or_else(|_| "{}".to_string()));
            }
            Err(_) => {
                // Fall through to the original logic
            }
        }
    }

    match serde_json::from_str::<Value>(params) {
        Ok(value) => {
            let sanitized = sanitize_value(value);
            Ok(serde_json::to_string(&sanitized).unwrap_or_else(|_| "{}".to_string()))
        }
        Err(e) => {
            let cleaned = clean_malformed_json(params);
            match serde_json::from_str::<Value>(&cleaned) {
                Ok(value) => {
                    let sanitized = sanitize_value(value);
                    Ok(serde_json::to_string(&sanitized).unwrap_or_else(|_| "{}".to_string()))
                }
                Err(_) => Err(format!("Invalid JSON parameters: {}", e)),
            }
        }
    }
}

/// Extracts the first complete JSON object from a string, ignoring trailing characters
#[allow(dead_code)] // Future use for JSON validation
fn extract_first_json_object(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if !trimmed.starts_with('{') {
        return None;
    }

    let mut brace_count = 0;
    let mut in_string = false;
    let mut escape_next = false;
    let mut end_pos = 0;

    for (i, ch) in trimmed.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => {
                escape_next = true;
            }
            '"' => {
                in_string = !in_string;
            }
            '{' if !in_string => {
                brace_count += 1;
            }
            '}' if !in_string => {
                brace_count -= 1;
                if brace_count == 0 {
                    end_pos = i + ch.len_utf8();
                    break;
                }
            }
            _ => {}
        }
    }

    if brace_count == 0 && end_pos > 0 {
        Some(trimmed[..end_pos].to_string())
    } else {
        None
    }
}

#[allow(dead_code)]
fn sanitize_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sanitized_map = Map::new();
            for (key, val) in map {
                let clean_key = sanitize_string(&key);
                if !clean_key.is_empty() {
                    sanitized_map.insert(clean_key, sanitize_value(val));
                }
            }
            Value::Object(sanitized_map)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(sanitize_value).collect()),
        Value::String(s) => Value::String(sanitize_string(&s)),
        _ => value,
    }
}

#[allow(dead_code)]
fn sanitize_string(s: &str) -> String {
    s.chars()
        .filter(|c| {
            c.is_ascii() || c.is_alphabetic() || c.is_numeric() || " .,!?;:-_()[]{}\"'".contains(*c)
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[allow(dead_code)]
fn clean_malformed_json(json_str: &str) -> String {
    let mut cleaned = json_str.to_string();

    cleaned = cleaned.replace("\\n", "\n");
    cleaned = cleaned.replace("\\t", "\t");
    cleaned = cleaned.replace("\\r", "\r");

    if !cleaned.trim_start().starts_with('{') && !cleaned.trim_start().starts_with('[') {
        cleaned = format!("{{{}}}", cleaned);
    }

    let mut brace_count = 0;
    let mut bracket_count = 0;
    let mut in_string = false;
    let mut escape_next = false;
    let mut result = String::new();

    for ch in cleaned.chars() {
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => {
                escape_next = true;
                result.push(ch);
            }
            '"' => {
                in_string = !in_string;
                result.push(ch);
            }
            '{' if !in_string => {
                brace_count += 1;
                result.push(ch);
            }
            '}' if !in_string => {
                if brace_count > 0 {
                    brace_count -= 1;
                }
                result.push(ch);
            }
            '[' if !in_string => {
                bracket_count += 1;
                result.push(ch);
            }
            ']' if !in_string => {
                if bracket_count > 0 {
                    bracket_count -= 1;
                }
                result.push(ch);
            }
            _ => {
                result.push(ch);
            }
        }
    }

    while brace_count > 0 {
        result.push('}');
        brace_count -= 1;
    }

    while bracket_count > 0 {
        result.push(']');
        bracket_count -= 1;
    }

    result
}

#[allow(dead_code)]
pub fn extract_error_context(error: &str) -> String {
    if error.contains("trailing characters") {
        "Parameter JSON contains extra characters after valid JSON. Check for unclosed quotes or brackets.".to_string()
    } else if error.contains("expected") {
        "Parameter JSON is malformed. Check syntax and structure.".to_string()
    } else if error.contains("invalid type") {
        "Parameter contains wrong data type. Check field types match expected schema.".to_string()
    } else {
        format!("JSON parsing error: {}", error)
    }
}
