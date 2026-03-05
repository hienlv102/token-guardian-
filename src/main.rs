mod cache;
mod config;
mod dict;
mod rtk;
mod server;
mod toon;

use anyhow::Result;
use rust_mcp_sdk::mcp_server::server_runtime;
use rust_mcp_sdk::schema::*;
use rust_mcp_sdk::{McpServer, StdioTransport, ToMcpServerHandler, TransportOptions};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::server::TokenGuardianHandler;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (stdout is used for MCP stdio)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("TokenGuardian MCP server starting...");

    // Load config
    let config = Config::load().unwrap_or_default();

    // Define server info
    let server_info = InitializeResult {
        server_info: Implementation {
            name: "token-guardian".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("TokenGuardian - Token Optimization Layer".into()),
            description: Some(
                "Reduces LLM token consumption by 60-90% through CLI filtering, \
                 JSON compression, dictionary encoding, and caching."
                    .into(),
            ),
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        protocol_version: ProtocolVersion::V2025_11_25.into(),
        instructions: Some(
            "TokenGuardian provides tools to reduce token usage:\n\
             - tg_smart_read: [RECOMMENDED] All-in-one file read with auto-detection, compression, and caching (replaces cache_get + read_file + compress_context + cache_set)\n\
             - tg_filter_command: Filter CLI output (ls, git, cat, etc.)\n\
             - tg_encode_json: Compress JSON arrays to TOON format\n\
             - tg_decode_toon: Decode TOON back to JSON\n\
             - tg_compress_context: Smart compress code/text (auto-detects content type)\n\
             - tg_cache_get/set/clear: Manual cache management\n\
             BEST PRACTICE: Use tg_smart_read instead of multiple separate calls."
                .into(),
        ),
        meta: None,
    };

    // Create transport and handler
    let transport = StdioTransport::new(TransportOptions::default())
        .map_err(|e| anyhow::anyhow!("Transport error: {:?}", e))?;
    let handler = TokenGuardianHandler::new(config);

    // Start the server
    let server = server_runtime::create_server(
        rust_mcp_sdk::mcp_server::McpServerOptions {
            server_details: server_info,
            transport,
            handler: handler.to_mcp_server_handler(),
            task_store: None,
            client_task_store: None,
        },
    );

    tracing::info!("TokenGuardian MCP server ready");
    server.start().await.map_err(|e| anyhow::anyhow!("Server error: {:?}", e))?;

    Ok(())
}
