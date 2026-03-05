use serde_json::Value;

/// Extract keys from a JSON object.
fn extract_keys(obj: &Value) -> Vec<String> {
    match obj {
        Value::Object(map) => map.keys().cloned().collect(),
        _ => vec![],
    }
}

/// Extract values from a JSON object in key order.
fn extract_values(obj: &Value, keys: &[String]) -> Vec<String> {
    keys.iter()
        .map(|key| match obj.get(key) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::Null) => "null".to_string(),
            Some(Value::Array(arr)) => {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .collect();
                format!("[{}]", items.join(";"))
            }
            Some(Value::Object(_)) => "[obj]".to_string(),
            None => "".to_string(),
        })
        .collect()
}

/// Escape commas and newlines in field values for TOON format.
fn escape_value(s: &str) -> String {
    if s.contains(',') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

/// Encode a JSON value to TOON (Token-Optimized Object Notation) format.
///
/// Arrays of objects with uniform schema get compressed to:
/// ```text
/// data[count]{key1,key2,...}:
///   val1,val2,...
///   val1,val2,...
/// ```
pub fn encode(json: &Value) -> String {
    match json {
        Value::Array(arr) if !arr.is_empty() => {
            // Check if all elements are objects with same keys
            let first_keys = extract_keys(&arr[0]);
            if first_keys.is_empty() {
                // Array of primitives
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .collect();
                return format!("list[{}]: {}", arr.len(), items.join(", "));
            }

            let uniform = arr.iter().all(|item| {
                let keys = extract_keys(item);
                keys == first_keys
            });

            if !uniform {
                return json.to_string();
            }

            let header = format!("data[{}]{{{}}}", arr.len(), first_keys.join(","));
            let rows: Vec<String> = arr
                .iter()
                .map(|obj| {
                    extract_values(obj, &first_keys)
                        .iter()
                        .map(|v| escape_value(v))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .collect();
            format!("{}:\n  {}", header, rows.join("\n  "))
        }
        Value::Array(_) => "data[0]{}:".to_string(),
        _ => json.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encode_array_of_objects() {
        let json = json!([
            {"id": "u001", "name": "Alice", "role": "admin", "active": true},
            {"id": "u002", "name": "Bob", "role": "user", "active": false}
        ]);
        let result = encode(&json);
        assert!(result.starts_with("data[2]{"));
        assert!(result.contains("u001"));
        assert!(result.contains("Alice"));
        assert!(result.contains("u002"));
        assert!(result.contains("Bob"));
    }

    #[test]
    fn test_encode_empty_array() {
        let json = json!([]);
        let result = encode(&json);
        assert_eq!(result, "data[0]{}:");
    }

    #[test]
    fn test_encode_single_object() {
        let json = json!({"key": "value"});
        let result = encode(&json);
        assert_eq!(result, json.to_string());
    }

    #[test]
    fn test_encode_array_of_primitives() {
        let json = json!([1, 2, 3, 4]);
        let result = encode(&json);
        assert!(result.starts_with("list[4]:"));
    }

    #[test]
    fn test_encode_values_with_commas() {
        let json = json!([
            {"name": "Alice, Bob", "age": 30}
        ]);
        let result = encode(&json);
        assert!(result.contains("\"Alice, Bob\""));
    }

    #[test]
    fn test_encode_preserves_token_reduction() {
        let json = json!([
            {"id": "u001", "name": "Alice", "role": "admin", "active": true},
            {"id": "u002", "name": "Bob", "role": "user", "active": false},
            {"id": "u003", "name": "Charlie", "role": "user", "active": true},
            {"id": "u004", "name": "Diana", "role": "admin", "active": true},
            {"id": "u005", "name": "Eve", "role": "user", "active": false}
        ]);
        let original = json.to_string();
        let encoded = encode(&json);
        // TOON should be shorter
        assert!(encoded.len() < original.len());
    }
}
