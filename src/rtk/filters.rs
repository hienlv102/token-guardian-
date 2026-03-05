use regex::Regex;

/// Filter `ls -la` style output to filenames only.
pub fn filter_ls_output(raw: &str) -> String {
    let mut files = Vec::new();
    let mut dirs = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("total ") {
            continue;
        }
        // ls -la format: permissions links owner group size date... name
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 9 {
            let name = parts[8..].join(" ");
            if name == "." || name == ".." {
                continue;
            }
            if trimmed.starts_with('d') {
                dirs.push(format!("{}/", name));
            } else {
                files.push(name);
            }
        } else if !trimmed.is_empty() {
            // Simple ls output (just filenames)
            files.push(trimmed.to_string());
        }
    }

    let mut result = String::new();
    if !dirs.is_empty() {
        result.push_str(&dirs.join(" "));
    }
    if !files.is_empty() {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(&files.join(" "));
    }
    result
}

/// Filter `cat` / file read output to function signatures and key lines.
pub fn filter_cat_output(raw: &str, query: Option<&str>) -> String {
    let lines: Vec<&str> = raw.lines().collect();

    // If file is small enough, return as-is
    if lines.len() <= 50 {
        return raw.to_string();
    }

    // Detect markdown files — use markdown-specific filtering
    let is_markdown = lines.iter().take(5).any(|l| l.starts_with('#'))
        && lines.iter().any(|l| l.starts_with("## ") || l.starts_with("### "));
    if is_markdown {
        return filter_markdown_file(raw);
    }

    let sig_re = Regex::new(
        r"^\s*(pub\s+)?(async\s+)?(fn|struct|enum|trait|impl|class|function|def|const|static|type|interface|export)\s+",
    )
    .unwrap();
    let import_re = Regex::new(r"^\s*(use |import |from |require\(|#include)").unwrap();

    let mut result = Vec::new();
    let mut context_lines: Vec<usize> = Vec::new();

    // Find relevant lines
    for (i, line) in lines.iter().enumerate() {
        if sig_re.is_match(line) || import_re.is_match(line) {
            context_lines.push(i);
        }
        if let Some(q) = query {
            if line.to_lowercase().contains(&q.to_lowercase()) {
                // Add surrounding context
                let start = i.saturating_sub(3);
                let end = (i + 3).min(lines.len() - 1);
                for j in start..=end {
                    context_lines.push(j);
                }
            }
        }
    }

    context_lines.sort();
    context_lines.dedup();

    if context_lines.is_empty() {
        // Fallback: first 30 + last 10 lines
        for line in lines.iter().take(30) {
            result.push(*line);
        }
        result.push("... (truncated)");
        for line in lines.iter().skip(lines.len().saturating_sub(10)) {
            result.push(*line);
        }
    } else {
        let mut prev: Option<usize> = None;
        for &idx in &context_lines {
            if let Some(p) = prev {
                if idx > p + 1 {
                    result.push("...");
                }
            }
            result.push(lines[idx]);
            prev = Some(idx);
        }
    }

    result.join("\n")
}

/// Filter `git log` output to oneline format.
pub fn filter_git_log(raw: &str) -> String {
    let mut entries = Vec::new();
    let mut current_hash = String::new();
    let mut current_msg = String::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("commit ") {
            if !current_hash.is_empty() {
                entries.push(format!(
                    "{} {}",
                    &current_hash[..7.min(current_hash.len())],
                    current_msg.trim()
                ));
            }
            current_hash = rest.to_string();
            current_msg.clear();
        } else if !trimmed.is_empty()
            && !trimmed.starts_with("Author:")
            && !trimmed.starts_with("Date:")
            && !trimmed.starts_with("Merge:")
        {
            if current_msg.is_empty() {
                current_msg = trimmed.to_string();
            }
        }
    }
    if !current_hash.is_empty() {
        entries.push(format!(
            "{} {}",
            &current_hash[..7.min(current_hash.len())],
            current_msg.trim()
        ));
    }

    // Already oneline format — detect and passthrough
    if entries.is_empty() && !raw.is_empty() {
        // Might already be --oneline, take first 10
        return raw.lines().take(10).collect::<Vec<_>>().join("\n");
    }

    entries.truncate(10);
    entries.join("\n")
}

/// Filter `git diff` output to compact summary.
pub fn filter_git_diff(raw: &str) -> String {
    let mut file_stats: Vec<String> = Vec::new();
    let mut current_file = String::new();
    let mut adds: usize = 0;
    let mut dels: usize = 0;
    let mut hunk_headers: Vec<String> = Vec::new();

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            // Save previous file
            if !current_file.is_empty() {
                file_stats.push(format!(
                    "{} +{} -{}",
                    current_file, adds, dels
                ));
            }
            // Extract b/filename
            if let Some(b_part) = rest.split(" b/").nth(1) {
                current_file = b_part.to_string();
            }
            adds = 0;
            dels = 0;
            hunk_headers.clear();
        } else if line.starts_with("@@") {
            hunk_headers.push(line.to_string());
        } else if line.starts_with('+') && !line.starts_with("+++") {
            adds += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            dels += 1;
        }
    }
    if !current_file.is_empty() {
        file_stats.push(format!("{} +{} -{}", current_file, adds, dels));
    }

    if file_stats.is_empty() {
        return raw.lines().take(20).collect::<Vec<_>>().join("\n");
    }

    let total_adds: usize = file_stats
        .iter()
        .filter_map(|s| {
            s.split('+').nth(1).and_then(|p| {
                p.split_whitespace()
                    .next()
                    .and_then(|n| n.parse::<usize>().ok())
            })
        })
        .sum();
    let total_dels: usize = file_stats
        .iter()
        .filter_map(|s| {
            s.split('-').last().and_then(|p| p.trim().parse::<usize>().ok())
        })
        .sum();

    let mut result = format!(
        "{} files changed, +{} -{}\n",
        file_stats.len(),
        total_adds,
        total_dels
    );
    for stat in &file_stats {
        result.push_str(&format!("  {}\n", stat));
    }
    result.trim_end().to_string()
}

/// Filter `git status` output to compact format.
pub fn filter_git_status(raw: &str) -> String {
    let mut modified = Vec::new();
    let mut added = Vec::new();
    let mut deleted = Vec::new();
    let mut untracked = Vec::new();
    let mut branch = String::new();

    let mut in_untracked = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("On branch ") {
            branch = trimmed.strip_prefix("On branch ").unwrap_or("").to_string();
        } else if trimmed.starts_with("Untracked files:") {
            in_untracked = true;
        } else if trimmed.starts_with("Changes not staged")
            || trimmed.starts_with("Changes to be committed")
        {
            in_untracked = false;
        } else if trimmed.starts_with("modified:") {
            modified.push(
                trimmed
                    .strip_prefix("modified:")
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            );
        } else if trimmed.starts_with("new file:") {
            added.push(
                trimmed
                    .strip_prefix("new file:")
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            );
        } else if trimmed.starts_with("deleted:") {
            deleted.push(
                trimmed
                    .strip_prefix("deleted:")
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            );
        } else if in_untracked
            && !trimmed.is_empty()
            && !trimmed.starts_with('(')
            && !trimmed.starts_with("no changes")
        {
            untracked.push(trimmed.to_string());
        }
    }

    let mut result = String::new();
    if !branch.is_empty() {
        result.push_str(&format!("branch: {}\n", branch));
    }
    if !modified.is_empty() {
        result.push_str(&format!("modified({}): {}\n", modified.len(), modified.join(", ")));
    }
    if !added.is_empty() {
        result.push_str(&format!("added({}): {}\n", added.len(), added.join(", ")));
    }
    if !deleted.is_empty() {
        result.push_str(&format!("deleted({}): {}\n", deleted.len(), deleted.join(", ")));
    }
    if !untracked.is_empty() {
        result.push_str(&format!(
            "untracked({}): {}\n",
            untracked.len(),
            untracked.join(", ")
        ));
    }
    if result.is_empty() {
        return "clean".to_string();
    }
    result.trim_end().to_string()
}

/// Filter generic command output by truncating.
pub fn filter_generic(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    let head = max_lines * 2 / 3;
    let tail = max_lines - head;

    let mut result: Vec<&str> = lines[..head].to_vec();
    result.push(&"... (truncated)");
    result.extend_from_slice(&lines[lines.len() - tail..]);
    result.join("\n")
}

/// Filter markdown file content to keep headings, first lines of sections, and collapse verbose parts.
pub fn filter_markdown_file(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let mut result = Vec::new();
    let mut in_code_block = false;
    let mut code_block_count = 0usize;
    let mut list_count = 0usize;

    for line in &lines {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                if code_block_count > 3 {
                    result.push(format!("  ... ({} more lines)", code_block_count - 3));
                }
                result.push("```".to_string());
                in_code_block = false;
                code_block_count = 0;
            } else {
                in_code_block = true;
                code_block_count = 0;
                result.push(line.to_string());
            }
            continue;
        }

        if in_code_block {
            code_block_count += 1;
            if code_block_count <= 3 {
                result.push(line.to_string());
            }
            continue;
        }

        // Always keep headings
        if trimmed.starts_with('#') {
            list_count = 0;
            result.push(line.to_string());
            continue;
        }

        // Collapse long lists
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            list_count += 1;
            if list_count <= 5 {
                result.push(line.to_string());
            } else if list_count == 6 {
                result.push("  ...".to_string());
            }
            continue;
        } else {
            list_count = 0;
        }

        // Keep tables but limit rows
        if trimmed.starts_with('|') {
            result.push(line.to_string());
            continue;
        }

        // Keep non-empty content lines
        if !trimmed.is_empty() {
            result.push(line.to_string());
        } else if result.last().map_or(true, |l| !l.is_empty()) {
            result.push(String::new());
        }
    }

    result.join("\n")
}

/// Detect command type from command string and apply appropriate filter.
pub fn filter_command_output(command: &str, output: &str) -> String {
    let cmd_lower = command.to_lowercase();
    let parts: Vec<&str> = cmd_lower.split_whitespace().collect();
    let base_cmd = parts.first().map(|s| *s).unwrap_or("");

    match base_cmd {
        "ls" => filter_ls_output(output),
        "cat" | "bat" | "less" | "head" | "tail" => filter_cat_output(output, None),
        "git" => {
            let sub = parts.get(1).map(|s| *s).unwrap_or("");
            match sub {
                "log" => filter_git_log(output),
                "diff" => filter_git_diff(output),
                "status" => filter_git_status(output),
                "add" | "commit" | "push" | "pull" => {
                    // Reduce success output to minimal
                    if output.len() < 200 {
                        output.to_string()
                    } else {
                        filter_generic(output, 5)
                    }
                }
                _ => filter_generic(output, 30),
            }
        }
        _ => filter_generic(output, 50),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_ls_output() {
        let input = "total 48
drwxr-xr-x  5 user staff  160 Jan 23 10:00 .
drwxr-xr-x  3 user staff   96 Jan 23 09:00 ..
-rw-r--r--  1 user staff 1234 Jan 23 10:00 Cargo.toml
-rw-r--r--  1 user staff  500 Jan 23 10:00 README.md
drwxr-xr-x  4 user staff  128 Jan 23 10:00 src";

        let result = filter_ls_output(input);
        assert!(result.contains("src/"));
        assert!(result.contains("Cargo.toml"));
        assert!(result.contains("README.md"));
        assert!(!result.contains("total"));
        assert!(!result.contains("drwxr-xr-x"));
    }

    #[test]
    fn test_filter_git_log() {
        let input = "commit abc1234567890
Author: User <user@example.com>
Date:   Mon Jan 1 10:00:00 2026

    Fix bug in parser

commit def4567890123
Author: User <user@example.com>
Date:   Sun Dec 31 09:00:00 2025

    Add new feature";

        let result = filter_git_log(input);
        assert!(result.contains("abc1234"));
        assert!(result.contains("Fix bug in parser"));
        assert!(result.contains("def4567"));
        assert!(!result.contains("Author:"));
        assert!(!result.contains("Date:"));
    }

    #[test]
    fn test_filter_git_status() {
        let input = "On branch main
Changes not staged for commit:
  (use \"git add <file>...\" to update what will be committed)

\tmodified:   src/main.rs
\tmodified:   Cargo.toml

Untracked files:
  (use \"git add <file>...\" to include in what will be committed)

\ttests/";

        let result = filter_git_status(input);
        assert!(result.contains("branch: main"));
        assert!(result.contains("modified(2)"));
        assert!(result.contains("untracked(1)"));
    }

    #[test]
    fn test_filter_git_diff() {
        let input = "diff --git a/src/main.rs b/src/main.rs
index 1234567..abcdefg 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,6 @@
 fn main() {
+    println!(\"hello\");
     let x = 1;
-    let y = 2;
+    let y = 3;
 }";

        let result = filter_git_diff(input);
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("+2"));
        assert!(result.contains("-1"));
    }

    #[test]
    fn test_filter_generic_short() {
        let input = "line1\nline2\nline3";
        assert_eq!(filter_generic(input, 10), input);
    }

    #[test]
    fn test_filter_generic_truncate() {
        let lines: Vec<String> = (0..100).map(|i| format!("line {}", i)).collect();
        let input = lines.join("\n");
        let result = filter_generic(&input, 15);
        assert!(result.contains("truncated"));
        assert!(result.lines().count() < 100);
    }

    #[test]
    fn test_filter_command_output_dispatch() {
        let ls_out = "total 4\n-rw-r--r-- 1 u g 100 Jan 1 10:00 file.rs";
        let result = filter_command_output("ls -la", ls_out);
        assert!(result.contains("file.rs"));
        assert!(!result.contains("total"));
    }
}
