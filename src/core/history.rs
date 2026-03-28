use serde::{Deserialize, Serialize};

/// A single entry in the clipboard history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipboardItem {
    /// Row ID assigned by SQLite (0 before first insert).
    pub id: i64,

    /// The actual clipboard text content.
    pub content: String,

    /// Unix timestamp (seconds since epoch) of when this was captured.
    pub timestamp: i64,

    /// Whether the user has pinned this item so it never gets evicted.
    pub pinned: bool,
}

impl ClipboardItem {
    /// Returns a display-friendly truncated preview (max 80 chars).
    pub fn preview(&self) -> String {
        let trimmed = self.content.trim();
        if trimmed.chars().count() > 80 {
            format!("{}…", trimmed.chars().take(79).collect::<String>())
        } else {
            trimmed.to_string()
        }
    }

    /// Returns a human-readable relative time string ("just now", "2 min ago", …).
    pub fn relative_time(&self) -> String {
        let now = chrono::Utc::now().timestamp();
        let delta = now - self.timestamp;

        match delta {
            0..=59 => "just now".to_string(),
            60..=3599 => format!("{} min ago", delta / 60),
            3600..=86399 => format!("{} hr ago", delta / 3600),
            _ => format!("{} days ago", delta / 86400),
        }
    }
}
