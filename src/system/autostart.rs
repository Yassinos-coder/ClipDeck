//! Generates and installs a systemd user service so ClipDeck starts
//! automatically on login.

use anyhow::{Context, Result};
use std::path::PathBuf;

const SERVICE_NAME: &str = "clipdeck.service";

/// Content of the systemd unit file.
fn service_content(executable: &str) -> String {
    format!(
        r#"[Unit]
Description=ClipDeck — clipboard manager & launcher
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart={executable}
Restart=on-failure
RestartSec=3
# Give GTK / X11 time to start up
Environment=DISPLAY=:0
Environment=DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/%U/bus

[Install]
WantedBy=graphical-session.target
"#,
        executable = executable
    )
}

/// Returns the path where the service file should be installed.
pub fn service_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("systemd")
        .join("user")
        .join(SERVICE_NAME)
}

/// Write the service file to `~/.config/systemd/user/clipdeck.service`.
///
/// `executable` should be the absolute path to the `clipdeck` binary
/// (e.g. `/usr/local/bin/clipdeck` or the result of `which clipdeck`).
pub fn install_service(executable: &str) -> Result<()> {
    let path = service_path();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create {:?}", parent))?;
    }

    std::fs::write(&path, service_content(executable))
        .with_context(|| format!("write {:?}", path))?;

    log::info!("systemd service written to {:?}", path);
    log::info!(
        "Enable autostart with:\n  systemctl --user daemon-reload\n  systemctl --user enable --now clipdeck.service"
    );

    Ok(())
}

/// Returns `true` if the service file already exists on disk.
pub fn is_installed() -> bool {
    service_path().exists()
}

/// Remove the service file (disables autostart, does not `systemctl disable`).
pub fn uninstall_service() -> Result<()> {
    let path = service_path();
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("remove {:?}", path))?;
        log::info!("systemd service removed from {:?}", path);
    }
    Ok(())
}
