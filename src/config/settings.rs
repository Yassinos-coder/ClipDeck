use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application-wide settings, persisted to ~/.config/clipdeck/settings.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Maximum clipboard entries to retain.
    pub max_history: usize,

    /// How often to poll the clipboard (milliseconds).
    pub poll_interval_ms: u64,

    /// Entries larger than this byte count are ignored.
    pub max_entry_size: usize,

    /// Substrings that trigger the "sensitive content" filter.
    pub ignored_patterns: Vec<String>,

    /// GitHub repo slug used for the version-check API call.
    pub github_repo: String,

    /// Compiled-in version string.
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_history: 50,
            poll_interval_ms: 500,
            max_entry_size: 1024 * 1024, // 1 MB
            ignored_patterns: vec![
                "password".to_string(),
                "passwd".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "api_key".to_string(),
                "apikey".to_string(),
                "private_key".to_string(),
            ],
            github_repo: "yourusername/clipdeck".to_string(),
            version: default_version(),
        }
    }
}

impl Settings {
    /// Path to the settings JSON file.
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("clipdeck")
            .join("settings.json")
    }

    /// Load settings from disk, falling back to defaults on any error.
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
                Err(e) => {
                    log::warn!("Could not read settings file: {e}");
                    Self::default()
                }
            }
        } else {
            let defaults = Self::default();
            // Best-effort save so the user can inspect/edit the file.
            let _ = defaults.save();
            defaults
        }
    }

    /// Persist settings to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Returns `true` if the text looks like sensitive content and should be skipped.
    pub fn is_sensitive(&self, content: &str) -> bool {
        let lower = content.to_lowercase();
        self.ignored_patterns.iter().any(|p| lower.contains(p))
    }
}
