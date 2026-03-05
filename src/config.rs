use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,

    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,

    #[serde(default = "default_max_lines")]
    pub max_output_lines: usize,

    #[serde(default)]
    pub default_lang: Option<String>,
}

fn default_cache_ttl() -> u64 {
    86400 // 24 hours
}

fn default_cache_dir() -> String {
    let cwd = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    cwd.join(".token_guardian").join(".cache")
        .to_string_lossy()
        .to_string()
}

fn default_max_lines() -> usize {
    50
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache_ttl_seconds: default_cache_ttl(),
            cache_dir: default_cache_dir(),
            max_output_lines: default_max_lines(),
            default_lang: None,
        }
    }
}

impl Config {
    /// Load config from `.tokenrules` file in the current directory,
    /// or fall back to `~/.config/token_guardian/config.toml`.
    pub fn load() -> Result<Self> {
        let candidates = [
            PathBuf::from(".tokenrules"),
            PathBuf::from(".tokenrules.toml"),
            dirs_config().join("config.toml"),
        ];

        for path in &candidates {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                let config: Config = toml::from_str(&content)?;
                return Ok(config);
            }
        }

        Ok(Config::default())
    }
}

fn dirs_config() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(format!("{}/.config/token_guardian", home))
}
