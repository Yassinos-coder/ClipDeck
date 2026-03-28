//! Shell shortcut definitions and execution.

use serde::{Deserialize, Serialize};

/// A user-defined shell shortcut stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    /// Row ID assigned by SQLite.
    pub id: i64,
    /// Human-readable display name.
    pub name: String,
    /// Shell command to execute (passed to `sh -c`).
    pub command: String,
    /// Unix timestamp of creation.
    pub created: i64,
}

impl Shortcut {
    /// Execute the shortcut command via `sh -c` in a blocking manner.
    ///
    /// Stdout and stderr are combined.  The result is truncated to 2000
    /// characters so the UI status label never overflows.
    pub fn execute(&self) -> anyhow::Result<String> {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .output()?;

        let mut combined = String::new();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() {
            combined.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(&stderr);
        }

        if combined.is_empty() {
            combined.push_str("(no output)");
        }

        // Truncate to 2000 chars to keep the UI manageable.
        if combined.len() > 2000 {
            combined.truncate(2000);
            combined.push_str("\n… (truncated)");
        }

        Ok(combined)
    }
}
