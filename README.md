# TokenGuardian MCP

> **Status: Experimental / Proof-of-Concept**

An MCP (Model Context Protocol) server that reduces token usage for AI coding assistants by compressing file content before it reaches the language model.

## Motivation

When AI coding assistants (Kilo Code, Cline, Cursor, etc.) read files, the full content is sent as tokens to the LLM. In multi-turn tasks, this adds up fast — especially on token-limited plans.

TokenGuardian sits between the IDE and the LLM as an MCP server, compressing file content using multiple strategies before it enters the context window.

## How It Works

```
IDE (read_file) → TokenGuardian MCP → Compressed content → LLM
```

The LLM receives a shorter representation with a small decoder header, preserving all semantic meaning while using fewer tokens.

## Compression Strategies

| Strategy | Target Content | Approach |
|----------|---------------|----------|
| **Dictionary substitution** | Code (Rust, JS/TS, Python) | Replace common patterns with short symbols (`println!` → `p!`, `function` → `fn`) |
| **Markdown summarization** | Documentation, READMEs | Keep headings + first lines, collapse long lists and code blocks |
| **TOON** | JSON/config | Compact JSON representation |
| **Log deduplication** | CLI output, logs | Collapse repeated lines |
| **Threshold gate** | All | Skip compression if savings < 5% to avoid adding overhead |

Content type is auto-detected — no manual configuration needed.

## Tools

| Tool | Purpose |
|------|---------|
| `tg_smart_read` | **All-in-one**: cache check → file read → auto-detect content type → best compression → cache result. Replaces 4 separate calls with 1. |
| `tg_compress_context` | Compress arbitrary text with auto-detected or specified strategy |
| `tg_cache_get` / `tg_cache_set` | Persistent cache (sled) for compressed content |
| `tg_read_file` | Read file with optional line filtering |

## Installation

### Prerequisites

- Rust 1.70+
- Cargo

### Build

```bash
git clone https://github.com/hienlv102/token-guardian-.git
cd token-guardian-
cargo build --release
```

Binary output: `target/release/token_guardian`

### MCP Configuration

Add to your IDE's MCP config (e.g., `.kilocode/mcp.json`, `cline_mcp_settings.json`):

```json
{
  "mcpServers": {
    "token_guardian": {
      "command": "/path/to/token_guardian",
      "args": [],
      "transportType": "stdio"
    }
  }
}
```

## Benchmarks

Measured on real-world files:

| Content Type | File Size | Compression | Notes |
|-------------|-----------|-------------|-------|
| Markdown README | ~5KB | 25-40% | Structural summarization + markdown dict |
| Rust source | ~3KB | 15-25% | Dictionary substitution |
| JSON config | ~2KB | 20-30% | TOON compact format |
| Small files (<500B) | <500B | Skipped | Threshold gate prevents negative compression |

> **Honest note**: For very small files or content with no matching dictionary patterns, TokenGuardian adds zero value. The threshold gate ensures it doesn't make things worse.

## Findings

### What works

- **`tg_smart_read`** — reducing 4 MCP round-trips to 1 is the biggest practical win
- **Markdown structural summarization** — collapsing code blocks and long lists saves significant tokens
- **Persistent cache** — avoids re-reading and re-compressing unchanged files across sessions

### What doesn't (limitations)

- **MCP tool-call overhead** — each MCP call has inherent JSON-RPC overhead. For very small files, this overhead can exceed the savings
- **Dictionary is static** — hand-curated patterns don't scale. Frequency-based dictionary generation would be better
- **Cannot control host context injection** — the dominant source of token waste in IDEs like Kilo Code is the repeated `environment_details` block injected every turn (~800-1500 tokens × N turns). No MCP tool can address this — it requires a fix at the IDE/host level

## Project Structure

```
token-guardian-/
├── src/
│   ├── main.rs              # Entry point
│   ├── server.rs            # MCP server + tool handlers
│   ├── cache/
│   │   └── store.rs         # Sled persistent cache
│   ├── dict/
│   │   ├── compressor.rs    # Dictionary compression engine
│   │   ├── static_dict.rs   # Language-specific dictionaries
│   │   └── summarizer.rs    # Markdown/log structural summarizer
│   ├── rtk/
│   │   ├── filters.rs       # Content filters (markdown, code, logs)
│   │   └── toon.rs          # TOON JSON compression
│   └── utils/
│       └── fs.rs            # File system utilities
├── tests/
│   └── integration_test.rs  # Integration tests
├── Cargo.toml
└── use-guide.md             # Agent-facing usage guide
```

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

## Contributing

This is an experimental project exploring MCP-based token optimization. Contributions, feedback, and issues are welcome.

## License

MIT