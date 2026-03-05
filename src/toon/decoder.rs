use anyhow::{Context, Result};
use serde_json::{json, Value};

/// Decode a TOON-encoded string back to JSON.
///
/// Expected format:
/// ```text
/// data[count]{key1,key2,...}:
///   val1,val2,...
///   val1,val2,...
/// ```
pub fn decode(toon: &str) -> Result<Value> {
    let trimmed = toon.trim();

    // Handle primitive list format: list[N]: val1, val2, ...
    if trimmed.starts_with("list[") {
        return decode_list(trimmed);
    }

    // Handle empty: data[0]{}:
    if trimmed.contains("[0]{}:") {
        return Ok(json!([]));
    }

    // Parse header: data[count]{key1,key2,...}:
    let colon_pos = trimmed
        .find("}:")
        .context("Invalid TOON: missing '}:' in header")?;
    let header = &trimmed[..colon_pos + 1];
    let body = trimmed[colon_pos + 2..].trim();

    // Extract keys from {key1,key2,...}
    let brace_start = header
        .find('{')
        .context("Invalid TOON: missing '{' in header")?;
    let keys_str = &header[brace_start + 1..header.len() - 1];
    let keys: Vec<&str> = keys_str.split(',').map(|s| s.trim()).collect();

    if keys.is_empty() || (keys.len() == 1 && keys[0].is_empty()) {
        return Ok(json!([]));
    }

    // Parse rows
    let mut result = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values = parse_csv_line(line);
        let mut obj = serde_json::Map::new();
        for (i, key) in keys.iter().enumerate() {
            let val_str = values.get(i).map(|s| s.as_str()).unwrap_or("");
            obj.insert(key.to_string(), infer_type(val_str));
        }
        result.push(Value::Object(obj));
    }

    Ok(Value::Array(result))
}

fn decode_list(toon: &str) -> Result<Value> {
    let colon_pos = toon.find(':').context("Invalid list TOON")?;
    let body = toon[colon_pos + 1..].trim();
    let items: Vec<Value> = body.split(", ").map(|s| infer_type(s.trim())).collect();
    Ok(Value::Array(items))
}

/// Parse a CSV line respecting quoted values.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' && !in_quotes {
            in_quotes = true;
        } else if ch == '"' && in_quotes {
            if chars.peek() == Some(&'"') {
                current.push('"');
                chars.next();
            } else {
                in_quotes = false;
            }
        } else if ch == ',' && !in_quotes {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

/// Infer JSON type from string value.
fn infer_type(s: &str) -> Value {
    match s {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        _ => {
            // Only parse as number if it doesn't have leading zeros (except "0" itself)
            let is_numeric = !s.is_empty()
                && (s == "0" || (!s.starts_with('0') && !s.starts_with("-0")))
                && s.chars()
                    .all(|c| c.is_ascii_digit() || c == '-' || c == '.');
            if is_numeric {
                if let Ok(n) = s.parse::<i64>() {
                    return json!(n);
                }
                if let Ok(n) = s.parse::<f64>() {
                    return json!(n);
                }
            }
            Value::String(s.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_decode_basic() {
        let toon = "data[2]{id,name,role,active}:\n  u001,Alice,admin,true\n  u002,Bob,user,false";
        let result = decode(toon).unwrap();
        let expected = json!([
            {"id": "u001", "name": "Alice", "role": "admin", "active": true},
            {"id": "u002", "name": "Bob", "role": "user", "active": false}
        ]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_empty() {
        let toon = "data[0]{}:";
        let result = decode(toon).unwrap();
        assert_eq!(result, json!([]));
    }

    #[test]
    fn test_decode_list() {
        let toon = "list[3]: 1, 2, 3";
        let result = decode(toon).unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_decode_quoted_commas() {
        let toon = "data[1]{name,age}:\n  \"Alice, Bob\",30";
        let result = decode(toon).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr[0]["name"], "Alice, Bob");
        assert_eq!(arr[0]["age"], 30);
    }

    #[test]
    fn test_roundtrip() {
        let original = json!([
            {"id": "001", "name": "Test", "count": 42, "active": true},
            {"id": "002", "name": "Demo", "count": 0, "active": false}
        ]);
        let encoded = crate::toon::encoder::encode(&original);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }
}
