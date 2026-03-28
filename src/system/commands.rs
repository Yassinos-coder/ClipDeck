//! Safe system-tool commands exposed in the Tools tab.
//!
//! Also provides X11-specific helpers for cursor position and window
//! positioning used by the popup window logic.

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

/// Get the current cursor position in root (screen) coordinates via x11rb.
/// Returns `None` if X11 is unavailable.
pub fn get_cursor_root_pos() -> Option<(i32, i32)> {
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::ConnectionExt as _;
    use x11rb::rust_connection::RustConnection;

    let (conn, screen_num) = RustConnection::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;
    let reply = conn.query_pointer(root).ok()?.reply().ok()?;
    Some((reply.root_x as i32, reply.root_y as i32))
}

/// Move the ClipDeck popup window to absolute screen position (x, y).
///
/// Uses `xdotool search --onlyvisible --name ClipDeck windowmove` — best-effort,
/// silently ignored if xdotool is unavailable.
pub fn move_clipdeck_window(x: i32, y: i32) {
    // First find the window XID, then move it.
    let Ok(out) = Command::new("xdotool")
        .args(["search", "--onlyvisible", "--name", "ClipDeck"])
        .output()
    else {
        return;
    };
    let ids = String::from_utf8_lossy(&out.stdout);
    if let Some(id) = ids.split_whitespace().next() {
        let _ = Command::new("xdotool")
            .args(["windowmove", id, &x.to_string(), &y.to_string()])
            .status();
    }
}

/// Save the current ClipDeck window position to `~/.config/clipdeck/window_pos`.
/// Silent no-op if xdotool is unavailable or the window is not found.
pub fn save_clipdeck_window_pos() {
    let Ok(out) = Command::new("xdotool")
        .args(["search", "--onlyvisible", "--name", "ClipDeck"])
        .output()
    else {
        return;
    };
    let ids = String::from_utf8_lossy(&out.stdout);
    let Some(id) = ids.split_whitespace().next() else {
        return;
    };
    let Ok(geo_out) = Command::new("xdotool")
        .args(["getwindowgeometry", "--shell", id])
        .output()
    else {
        return;
    };
    let geo = String::from_utf8_lossy(&geo_out.stdout);
    let mut wx: Option<i32> = None;
    let mut wy: Option<i32> = None;
    for line in geo.lines() {
        if let Some(v) = line.strip_prefix("X=") { wx = v.parse().ok(); }
        if let Some(v) = line.strip_prefix("Y=") { wy = v.parse().ok(); }
    }
    if let (Some(x), Some(y)) = (wx, wy) {
        if let Some(cfg) = dirs::config_dir() {
            let dir = cfg.join("clipdeck");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("window_pos"), format!("{x} {y}"));
        }
    }
}

/// Load a previously saved window position from `~/.config/clipdeck/window_pos`.
pub fn load_clipdeck_window_pos() -> Option<(i32, i32)> {
    let path = dirs::config_dir()?.join("clipdeck/window_pos");
    let content = std::fs::read_to_string(path).ok()?;
    let mut parts = content.split_whitespace();
    let x: i32 = parts.next()?.parse().ok()?;
    let y: i32 = parts.next()?.parse().ok()?;
    Some((x, y))
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
