//! Safe system-tool commands exposed in the Tools tab.

use anyhow::{bail, Result};
use std::process::Command;

/// Flush the systemd-resolved DNS cache.
/// Requires `systemd-resolve` to be installed (default on Ubuntu/Pop!_OS).
pub fn flush_dns() -> Result<String> {
    let out = Command::new("systemd-resolve")
        .arg("--flush-caches")
        .output()?;

    if out.status.success() {
        Ok("DNS cache flushed successfully.".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("flush-caches failed: {stderr}")
    }
}

/// Return total / used / free memory in a human-readable string.
pub fn memory_info() -> Result<String> {
    let out = Command::new("free").arg("-h").output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        bail!("free command failed")
    }
}

/// Open the system Nautilus file manager at the given path (or home dir).
pub fn open_file_manager(path: Option<&str>) -> Result<()> {
    let target = path.unwrap_or(".");
    Command::new("xdg-open").arg(target).spawn()?;
    Ok(())
}

/// Simulate a Ctrl+V key press via xdotool so the selected clipboard item
/// gets pasted into the previously focused window.
///
/// This is a best-effort operation — xdotool may not be installed.
pub fn simulate_paste() -> Result<()> {
    let status = Command::new("xdotool")
        .args(["key", "--clearmodifiers", "ctrl+v"])
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => bail!("xdotool exited with non-zero status"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // xdotool not installed — not a hard error
            log::debug!("xdotool not found; skipping auto-paste");
            Ok(())
        }
        Err(e) => bail!("xdotool error: {e}"),
    }
}
