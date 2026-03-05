use std::collections::HashMap;

use super::static_dict;

/// Minimum compression ratio (percentage of bytes saved) to justify adding the dictionary header.
/// If compression saves less than this, return the original text unchanged to avoid overhead.
const MIN_COMPRESSION_RATIO: f64 = 5.0;

#[allow(dead_code)]
pub struct DictCompressor {
    /// Mappings sorted by key length (longest first) for correct replacement order.
    forward: Vec<(String, String)>,
    /// Reverse mappings for decompression.
    reverse: Vec<(String, String)>,
    /// Only the mappings that were actually used during compression.
    used_mappings: std::cell::RefCell<Vec<(String, String)>>,
}

impl DictCompressor {
    /// Create a new compressor for the given language.
    pub fn new(lang: Option<&str>) -> Self {
        let dict = static_dict::get_dict(lang);
        Self::from_map(dict)
    }

    /// Create a compressor from an arbitrary mapping.
    pub fn from_map(map: HashMap<String, String>) -> Self {
        // Sort by key length descending to replace longest matches first
        let mut forward: Vec<(String, String)> = map.into_iter().collect();
        forward.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        let reverse: Vec<(String, String)> = forward
            .iter()
            .map(|(k, v)| (v.clone(), k.clone()))
            .collect();

        Self {
            forward,
            reverse,
            used_mappings: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Compress text using the dictionary.
    /// Tracks which mappings were actually used for a minimal decoder prompt.
    pub fn compress(&self, text: &str) -> String {
        let mut result = text.to_string();
        let mut used = Vec::new();
        for (long, short) in &self.forward {
            if result.contains(long.as_str()) {
                result = result.replace(long.as_str(), short);
                used.push((long.clone(), short.clone()));
            }
        }
        *self.used_mappings.borrow_mut() = used;
        result
    }

    /// Compress text with threshold check.
    /// Returns None if compression ratio is below MIN_COMPRESSION_RATIO (not worth the overhead).
    /// Returns Some((decoder_prompt, compressed_text)) if compression is effective.
    pub fn compress_with_threshold(&self, text: &str) -> Option<(String, String)> {
        let compressed = self.compress(text);
        let original_len = text.len() as f64;
        let compressed_len = compressed.len() as f64;

        if original_len == 0.0 {
            return None;
        }

        let ratio = ((original_len - compressed_len) / original_len) * 100.0;

        if ratio < MIN_COMPRESSION_RATIO {
            // Compression not worth it — would add overhead for negligible savings
            None
        } else {
            let prompt = self.decoder_prompt_used_only();
            Some((prompt, compressed))
        }
    }

    /// Decompress text back to original form.
    #[allow(dead_code)]
    pub fn decompress(&self, text: &str) -> String {
        let mut result = text.to_string();
        // Reverse in opposite order (shortest abbreviations first to avoid conflicts)
        for (short, long) in self.reverse.iter().rev() {
            result = result.replace(short.as_str(), long);
        }
        result
    }

    /// Generate a decoder prompt including ONLY the mappings that were actually used.
    /// This avoids sending useless dictionary entries to the LLM.
    pub fn decoder_prompt_used_only(&self) -> String {
        let used = self.used_mappings.borrow();
        if used.is_empty() {
            return String::new();
        }
        let mappings: Vec<String> = used
            .iter()
            .map(|(long, short)| format!("{}={}", short, long))
            .collect();
        format!("[Dict: {}]", mappings.join(", "))
    }

    /// Generate a decoder prompt that teaches the LLM the abbreviations used.
    #[allow(dead_code)]
    pub fn decoder_prompt(&self) -> String {
        let mappings: Vec<String> = self
            .forward
            .iter()
            .take(20) // Limit to keep prompt small
            .map(|(long, short)| format!("{}={}", short, long))
            .collect();
        format!("[Dict: {}]", mappings.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_rust() {
        let comp = DictCompressor::new(Some("rust"));
        let input = "pub fn main() { println!(\"hello\"); }";
        let result = comp.compress(input);
        assert!(result.contains("pfn"));
        assert!(result.contains("p!"));
        assert!(result.len() < input.len());
    }

    #[test]
    fn test_compress_js() {
        let comp = DictCompressor::new(Some("js"));
        let input = "console.log(\"test\"); export default App;";
        let result = comp.compress(input);
        assert!(result.contains("cl"));
        assert!(result.contains("ed"));
    }

    #[test]
    fn test_compress_prompts() {
        let comp = DictCompressor::new(None);
        let input = "Please implement a function that handles error message parsing";
        let result = comp.compress(input);
        assert!(result.contains("Impl"));
        assert!(result.contains("errmsg"));
    }

    #[test]
    fn test_decoder_prompt() {
        let comp = DictCompressor::new(Some("rust"));
        let prompt = comp.decoder_prompt();
        assert!(prompt.starts_with("[Dict:"));
        assert!(prompt.contains("="));
    }

    #[test]
    fn test_no_compress_unrelated() {
        let comp = DictCompressor::new(Some("rust"));
        let input = "Hello World 12345";
        let result = comp.compress(input);
        assert_eq!(result, input);
    }
}
