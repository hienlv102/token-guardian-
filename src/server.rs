use async_trait::async_trait;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::*;
use rust_mcp_sdk::McpServer;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::cache::store::ReasoningCache;
use crate::config::Config;
use crate::dict::compressor::DictCompressor;
use crate::dict::summarizer;
use crate::rtk::filters;
use crate::toon::{decoder, encoder};

pub struct TokenGuardianHandler {
    _config: Config,
    cache: Option<ReasoningCache>,
}

impl TokenGuardianHandler {
    pub fn new(config: Config) -> Self {
        // Ensure the cache directory exists
        if let Err(e) = std::fs::create_dir_all(&config.cache_dir) {
            eprintln!("Failed to create cache directory {}: {}", config.cache_dir, e);
        }

        let cache = ReasoningCache::new(&config.cache_dir, config.cache_ttl_seconds).ok();
        Self { _config: config, cache }
    }
}

fn make_tool(name: &str, description: &str, params: serde_json::Value) -> Tool {
    // Convert params JSON object to the HashMap<String, Map<String, Value>> format
    let properties: Option<HashMap<String, serde_json::Map<String, serde_json::Value>>> =
        params.as_object().map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| {
                    v.as_object().map(|prop_obj| (k.clone(), prop_obj.clone()))
                })
                .collect()
        });

    let required: Vec<String> = params
        .as_object()
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    Tool {
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: ToolInputSchema::new(required, properties, None),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

#[async_trait]
impl ServerHandler for TokenGuardianHandler {
    async fn handle_list_tools_request(
        &self,
        _request: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        let tools = vec![
            make_tool(
                "tg_filter_command",
                "Filter verbose CLI command output to reduce tokens. Provide the command name and its raw output.",
                json!({
                    "command": {
                        "type": "string",
                        "description": "The CLI command that was run (e.g. 'ls -la', 'git status', 'git log')"
                    },
                    "output": {
                        "type": "string",
                        "description": "The raw output from the command"
                    }
                }),
            ),
            make_tool(
                "tg_encode_json",
                "Compress JSON arrays into TOON (Token-Optimized Object Notation) format to reduce tokens.",
                json!({
                    "json": {
                        "type": "string",
                        "description": "JSON string to encode (works best with arrays of objects)"
                    }
                }),
            ),
            make_tool(
                "tg_decode_toon",
                "Decode a TOON-encoded string back to JSON.",
                json!({
                    "toon": {
                        "type": "string",
                        "description": "TOON-encoded string to decode back to JSON"
                    }
                }),
            ),
            make_tool(
                "tg_compress_context",
                "Compress code/text using dictionary abbreviations to reduce tokens. Returns compressed text with a decoder prompt.",
                json!({
                    "text": {
                        "type": "string",
                        "description": "Text or code to compress"
                    },
                    "lang": {
                        "type": "string",
                        "description": "Programming language hint: rust, js, ts, python (optional)"
                    }
                }),
            ),
            make_tool(
                "tg_cache_get",
                "Retrieve a cached result by task description and file paths.",
                json!({
                    "task": {
                        "type": "string",
                        "description": "Task description used as cache key"
                    },
                    "files": {
                        "type": "string",
                        "description": "Comma-separated file paths for cache key (optional)"
                    }
                }),
            ),
            make_tool(
                "tg_cache_set",
                "Store a result in cache for later retrieval.",
                json!({
                    "task": {
                        "type": "string",
                        "description": "Task description used as cache key"
                    },
                    "files": {
                        "type": "string",
                        "description": "Comma-separated file paths for cache key (optional)"
                    },
                    "value": {
                        "type": "string",
                        "description": "The result to cache"
                    }
                }),
            ),
            make_tool(
                "tg_cache_clear",
                "Clear all cached reasoning data.",
                json!({}),
            ),
            make_tool(
                "tg_smart_read",
                "All-in-one tool: checks cache, reads file, auto-detects content type, applies best compression strategy (structural summarization for markdown, dict compression for code, TOON for JSON). Returns compressed content or cached result. Use this INSTEAD of separate cache_get + read_file + compress_context calls.",
                json!({
                    "task": {
                        "type": "string",
                        "description": "Task description (used as cache key)"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "File path to read and compress"
                    },
                    "lang": {
                        "type": "string",
                        "description": "Language hint: rust, js, ts, python, md (optional, auto-detected if omitted)"
                    }
                }),
            ),
        ];

        Ok(ListToolsResult {
            tools,
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let args = params.arguments.as_ref().cloned().unwrap_or_default();

        let result = match params.name.as_str() {
            "tg_filter_command" => {
                let command = get_str(&args, "command").unwrap_or_default();
                let output = get_str(&args, "output").unwrap_or_default();
                filters::filter_command_output(&command, &output)
            }

            "tg_encode_json" => {
                let json_str = get_str(&args, "json").unwrap_or_default();
                match serde_json::from_str::<serde_json::Value>(&json_str) {
                    Ok(val) => encoder::encode(&val),
                    Err(e) => format!("Error parsing JSON: {}", e),
                }
            }

            "tg_decode_toon" => {
                let toon_str = get_str(&args, "toon").unwrap_or_default();
                match decoder::decode(&toon_str) {
                    Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_default(),
                    Err(e) => format!("Error decoding TOON: {}", e),
                }
            }

            "tg_compress_context" => {
                let text = get_str(&args, "text").unwrap_or_default();
                let lang = get_str(&args, "lang");
                smart_compress(&text, lang.as_deref())
            }

            "tg_smart_read" => {
                let task = get_str(&args, "task").unwrap_or_default();
                let file_path = get_str(&args, "file_path").unwrap_or_default();
                let lang = get_str(&args, "lang");

                // Step 1: Check cache
                let cache_key = ReasoningCache::make_key(&task, &[file_path.as_str()]);
                if let Some(cache) = &self.cache {
                    if let Some(cached) = cache.get(&cache_key) {
                        return Ok(CallToolResult::text_content(vec![TextContent::new(
                            format!("CACHE_HIT\n{}", cached),
                            None,
                            None,
                        )]));
                    }
                }

                // Step 2: Read file
                let content = match std::fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(e) => return Ok(CallToolResult::text_content(vec![TextContent::new(
                        format!("ERROR: Cannot read file '{}': {}", file_path, e),
                        None,
                        None,
                    )])),
                };

                // Step 3: Smart compress based on content type
                let compressed = smart_compress(&content, lang.as_deref());

                // Step 4: Auto-cache the compressed result
                if let Some(cache) = &self.cache {
                    let _ = cache.set(&cache_key, &compressed);
                }

                format!("CACHE_MISS\n{}", compressed)
            }

            "tg_cache_get" => {
                let task = get_str(&args, "task").unwrap_or_default();
                let files_str = get_str(&args, "files").unwrap_or_default();
                let files: Vec<&str> = if files_str.is_empty() {
                    vec![]
                } else {
                    files_str.split(',').map(|s| s.trim()).collect()
                };
                let key = ReasoningCache::make_key(&task, &files);
                match &self.cache {
                    Some(cache) => cache.get(&key).unwrap_or_else(|| "CACHE_MISS".to_string()),
                    None => "CACHE_UNAVAILABLE".to_string(),
                }
            }

            "tg_cache_set" => {
                let task = get_str(&args, "task").unwrap_or_default();
                let files_str = get_str(&args, "files").unwrap_or_default();
                let value = get_str(&args, "value").unwrap_or_default();
                let files: Vec<&str> = if files_str.is_empty() {
                    vec![]
                } else {
                    files_str.split(',').map(|s| s.trim()).collect()
                };
                let key = ReasoningCache::make_key(&task, &files);
                match &self.cache {
                    Some(cache) => match cache.set(&key, &value) {
                        Ok(_) => "CACHED".to_string(),
                        Err(e) => format!("CACHE_ERROR: {}", e),
                    },
                    None => "CACHE_UNAVAILABLE".to_string(),
                }
            }

            "tg_cache_clear" => match &self.cache {
                Some(cache) => match cache.clear() {
                    Ok(_) => "CACHE_CLEARED".to_string(),
                    Err(e) => format!("CACHE_ERROR: {}", e),
                },
                None => "CACHE_UNAVAILABLE".to_string(),
            },

            _ => return Err(CallToolError::unknown_tool(params.name)),
        };

        Ok(CallToolResult::text_content(vec![TextContent::new(
            result, None, None,
        )]))
    }
}

fn get_str(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Smart compression: auto-detects content type and applies the best strategy.
/// 1. For Markdown: structural summarization (collapse lists, code blocks, tables)
/// 2. For JSON: TOON encoding
/// 3. For Code: dictionary compression with threshold check
/// 4. For Logs: deduplication-based compression
/// 5. Fallback: dictionary compression with threshold check
fn smart_compress(text: &str, lang_hint: Option<&str>) -> String {
    let content_type = match lang_hint {
        Some("md" | "markdown") => summarizer::ContentType::Markdown,
        Some("json") => summarizer::ContentType::Json,
        Some("rust" | "rs" | "js" | "ts" | "javascript" | "typescript" | "python" | "py") => {
            summarizer::ContentType::Code
        }
        _ => summarizer::detect_content_type(text),
    };

    match content_type {
        summarizer::ContentType::Markdown => {
            // Try structural summarization first
            if let Some(structural) = summarizer::compress_markdown(text) {
                // Then try dict compression on the already-shortened text
                let compressor = DictCompressor::new(Some("md"));
                match compressor.compress_with_threshold(&structural) {
                    Some((prompt, compressed)) => format!("{}\n---\n{}", prompt, compressed),
                    None => structural,
                }
            } else {
                // Short markdown — try dict compression only
                let compressor = DictCompressor::new(Some("md"));
                match compressor.compress_with_threshold(text) {
                    Some((prompt, compressed)) => format!("{}\n---\n{}", prompt, compressed),
                    None => text.to_string(),
                }
            }
        }
        summarizer::ContentType::Json => {
            // Try TOON encoding
            match serde_json::from_str::<serde_json::Value>(text) {
                Ok(val) => {
                    let encoded = encoder::encode(&val);
                    if encoded.len() < text.len() {
                        encoded
                    } else {
                        text.to_string()
                    }
                }
                Err(_) => text.to_string(),
            }
        }
        summarizer::ContentType::LogOutput => {
            if let Some(compressed) = summarizer::compress_log_output(text) {
                compressed
            } else {
                text.to_string()
            }
        }
        summarizer::ContentType::Code | _ => {
            let lang = lang_hint.or(Some("rust")); // default for code
            let compressor = DictCompressor::new(lang);
            match compressor.compress_with_threshold(text) {
                Some((prompt, compressed)) => format!("{}\n---\n{}", prompt, compressed),
                None => text.to_string(),
            }
        }
    }
}
