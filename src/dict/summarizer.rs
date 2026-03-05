/// Structural summarization for content that doesn't benefit from dictionary compression.
/// This module extracts the essential structure from markdown, configs, and other non-code content
/// to dramatically reduce tokens while preserving semantic meaning.

use regex::Regex;

/// Maximum lines before structural summarization kicks in.
const STRUCTURAL_THRESHOLD: usize = 40;

/// Structurally compress markdown content by extracting headings, key lines, and collapsing verbose sections.
pub fn compress_markdown(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();

    // For short markdown, not worth summarizing
    if lines.len() <= STRUCTURAL_THRESHOLD {
        return None;
    }

    let mut result = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_lines = 0usize;
    let mut prev_was_blank = false;
    let mut consecutive_list_items = 0usize;

    for line in &lines {
        let trimmed = line.trim();

        // Track code blocks
        if trimmed.starts_with("```") {
            if in_code_block {
                // End of code block — emit summary
                if code_block_lines > 3 {
                    result.push(format!("  [{}L {}]", code_block_lang, code_block_lines));
                }
                result.push("```".to_string());
                in_code_block = false;
                code_block_lines = 0;
                continue;
            } else {
                in_code_block = true;
                code_block_lang = trimmed.strip_prefix("```").unwrap_or("").to_string();
                result.push(line.to_string());
                // Include first 3 lines of code block
                code_block_lines = 0;
                continue;
            }
        }

        if in_code_block {
            code_block_lines += 1;
            if code_block_lines <= 3 {
                result.push(line.to_string());
            }
            continue;
        }

        // Always keep headings
        if trimmed.starts_with('#') {
            if !prev_was_blank && !result.is_empty() {
                result.push(String::new());
            }
            result.push(line.to_string());
            prev_was_blank = false;
            consecutive_list_items = 0;
            continue;
        }

        // Keep table headers and separators, but limit table rows
        if trimmed.starts_with('|') {
            if trimmed.contains("---") {
                result.push(line.to_string());
                consecutive_list_items = 0;
            } else if consecutive_list_items < 5 {
                result.push(line.to_string());
                consecutive_list_items += 1;
            } else if consecutive_list_items == 5 {
                result.push("| ... |".to_string());
                consecutive_list_items += 1;
            }
            prev_was_blank = false;
            continue;
        }

        // Keep list items but collapse long lists
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || is_ordered_list_item(trimmed)
        {
            consecutive_list_items += 1;
            if consecutive_list_items <= 6 {
                result.push(line.to_string());
            } else if consecutive_list_items == 7 {
                result.push("  ... (more items)".to_string());
            }
            prev_was_blank = false;
            continue;
        } else {
            consecutive_list_items = 0;
        }

        // Keep bold/important lines
        if trimmed.starts_with("**") || trimmed.starts_with(">[") {
            result.push(line.to_string());
            prev_was_blank = false;
            continue;
        }

        // Keep non-empty paragraph lines (first line of paragraph)
        if !trimmed.is_empty() {
            if prev_was_blank || result.is_empty() {
                result.push(line.to_string());
            }
            prev_was_blank = false;
        } else {
            if !prev_was_blank {
                result.push(String::new());
            }
            prev_was_blank = true;
        }
    }

    let compressed = result.join("\n");

    // Only return if we actually achieved meaningful compression
    let ratio = 1.0 - (compressed.len() as f64 / text.len() as f64);
    if ratio > 0.1 {
        Some(compressed)
    } else {
        None
    }
}

/// Detect auto-detectable content type from text content.
pub fn detect_content_type(text: &str) -> ContentType {
    let first_lines: Vec<&str> = text.lines().take(10).collect();
    let first_chunk = first_lines.join("\n");

    // Check for markdown
    let md_signals = first_chunk.starts_with('#')
        || first_chunk.contains("\n## ")
        || first_chunk.contains("\n### ")
        || (first_chunk.contains("```") && first_chunk.contains("- **"));

    if md_signals {
        return ContentType::Markdown;
    }

    // Check for JSON
    let trimmed = text.trim();
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        return ContentType::Json;
    }

    // Check for TOML/INI config
    if first_chunk.contains("[package]")
        || first_chunk.contains("[dependencies]")
        || first_chunk.contains("[profile")
    {
        return ContentType::Config;
    }

    // Check for code patterns
    let code_re =
        Regex::new(r"^\s*(pub\s+)?(fn|struct|enum|impl|class|function|def|import|use|const)\s")
            .unwrap();
    let code_line_count = first_lines
        .iter()
        .filter(|l| code_re.is_match(l))
        .count();
    if code_line_count >= 2 {
        return ContentType::Code;
    }

    // Check for CLI/log output
    let has_timestamps = Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}").unwrap();
    if first_lines
        .iter()
        .filter(|l| has_timestamps.is_match(l))
        .count()
        >= 2
    {
        return ContentType::LogOutput;
    }

    ContentType::Unknown
}

#[derive(Debug, PartialEq)]
pub enum ContentType {
    Markdown,
    Json,
    Config,
    Code,
    LogOutput,
    Unknown,
}

fn is_ordered_list_item(s: &str) -> bool {
    let re = Regex::new(r"^\d+\.\s").unwrap();
    re.is_match(s)
}

/// Compress log/CLI output by deduplicating repeated patterns and keeping unique lines.
pub fn compress_log_output(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= 30 {
        return None;
    }

    let mut seen_patterns = std::collections::HashSet::new();
    let mut result = Vec::new();
    let mut dedup_count = 0usize;

    // Normalize lines by stripping timestamps/IDs/numbers for pattern detection
    let normalize_re =
        Regex::new(r"(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}[.\d]*\s*|\b\d+\b)").unwrap();

    for line in &lines {
        let normalized = normalize_re.replace_all(line, "N").to_string();
        let pattern = if normalized.len() > 50 {
            normalized[..50].to_string()
        } else {
            normalized.clone()
        };

        if seen_patterns.contains(&pattern) {
            dedup_count += 1;
        } else {
            if dedup_count > 0 {
                result.push(format!("  [... {} similar lines]", dedup_count));
                dedup_count = 0;
            }
            seen_patterns.insert(pattern);
            result.push(line.to_string());
        }
    }

    if dedup_count > 0 {
        result.push(format!("  [... {} similar lines]", dedup_count));
    }

    let compressed = result.join("\n");
    let ratio = 1.0 - (compressed.len() as f64 / text.len() as f64);
    if ratio > 0.1 {
        Some(compressed)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_markdown() {
        let md = "# Title\n\nSome text\n\n## Section\n\n- item 1\n- item 2\n";
        assert_eq!(detect_content_type(md), ContentType::Markdown);
    }

    #[test]
    fn test_detect_json() {
        let json = r#"[{"id": 1, "name": "test"}]"#;
        assert_eq!(detect_content_type(json), ContentType::Json);
    }

    #[test]
    fn test_detect_code() {
        let code = "use std::io;\nfn main() {\n    println!(\"hello\");\n}\n";
        assert_eq!(detect_content_type(code), ContentType::Code);
    }

    #[test]
    fn test_compress_short_markdown() {
        let short_md = "# Title\nSome text\n";
        assert!(compress_markdown(short_md).is_none());
    }

    #[test]
    fn test_compress_long_markdown() {
        let mut md = String::from("# Title\n\nDescription\n\n## Section 1\n\n");
        for i in 0..50 {
            md.push_str(&format!("- Item {} with some longer description text\n", i));
        }
        md.push_str("\n## Section 2\n\nMore content\n");

        let compressed = compress_markdown(&md);
        assert!(compressed.is_some());
        let compressed = compressed.unwrap();
        assert!(compressed.len() < md.len());
        assert!(compressed.contains("# Title"));
        assert!(compressed.contains("## Section 1"));
        assert!(compressed.contains("... (more items)"));
    }

    #[test]
    fn test_compress_markdown_with_code_blocks() {
        let mut md = String::from("# Guide\n\n## Install\n\n```bash\n");
        for i in 0..20 {
            md.push_str(&format!("step {} --do-something-long\n", i));
        }
        md.push_str("```\n\n## Usage\n\nSome usage text\n");
        // Pad to exceed threshold
        for i in 0..30 {
            md.push_str(&format!("- Feature {}\n", i));
        }

        let compressed = compress_markdown(&md);
        assert!(compressed.is_some());
        let c = compressed.unwrap();
        // Should keep first 3 code lines and summarize rest
        assert!(c.contains("step 0"));
        assert!(c.contains("step 1"));
        assert!(c.contains("step 2"));
        assert!(!c.contains("step 10"));
    }

    #[test]
    fn test_compress_log_output() {
        let mut log = String::new();
        for i in 0..100 {
            log.push_str(&format!(
                "2026-03-05T10:{}:00 INFO  Processing request {}\n",
                i % 60,
                i
            ));
        }
        let compressed = compress_log_output(&log);
        assert!(compressed.is_some());
        assert!(compressed.unwrap().len() < log.len());
    }
}
