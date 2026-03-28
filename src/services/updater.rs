//! Auto-updater: check GitHub releases, download the new binary, replace the
//! running exe, and restart the process (via systemd or exec).
//!
//! Release asset naming convention expected on GitHub:
//!   `clipdeck-linux-x86_64`   (preferred)
//!   `clipdeck`                (fallback)
//!
//! The database at ~/.local/share/clipdeck/history.db is never touched.

use std::os::unix::fs::PermissionsExt as _;

use serde::Deserialize;
use tokio::io::AsyncWriteExt as _;

// ── GitHub API types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// Metadata about an available release.
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub tag: String,
    pub download_url: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Check GitHub releases and return release info if a newer version exists.
/// Returns `Ok(None)` when the installed version is up to date.
pub async fn check_for_update(
    github_repo: &str,
    current_version: &str,
) -> anyhow::Result<Option<ReleaseInfo>> {
    let url = format!("https://api.github.com/repos/{github_repo}/releases/latest");

    let client = reqwest::Client::builder()
        .user_agent(concat!("clipdeck/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let release: GithubRelease = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let remote = release.tag_name.trim_start_matches('v');
    let local = current_version.trim_start_matches('v');

    if !is_newer(remote, local) {
        return Ok(None);
    }

    // Find the Linux binary asset.
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.contains("linux") && a.name.contains("x86_64"))
        .or_else(|| release.assets.iter().find(|a| a.name == "clipdeck"))
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".tar.gz")));

    match asset {
        Some(a) => Ok(Some(ReleaseInfo {
            tag: release.tag_name,
            download_url: a.browser_download_url.clone(),
        })),
        None => {
            // Release exists but no binary asset — report version only.
            log::warn!(
                "Release {} found but no binary asset detected. \
                 Build from source: https://github.com/{github_repo}",
                release.tag_name
            );
            Ok(Some(ReleaseInfo {
                tag: release.tag_name,
                download_url: String::new(), // empty signals source-only release
            }))
        }
    }
}

/// Download the release binary, replace the running exe, and restart.
///
/// `on_progress` is called with human-readable status strings so the UI can
/// update a progress label.  This function does **not** return on success
/// (it execs/exits the process).
pub async fn download_install_restart(
    info: &ReleaseInfo,
    on_progress: impl Fn(String) + Send + 'static,
) -> anyhow::Result<()> {
    if info.download_url.is_empty() {
        anyhow::bail!(
            "No binary asset in release {}. Please rebuild from source.",
            info.tag
        );
    }

    let current_exe = std::env::current_exe()?;
    // Download to /tmp — the current exe may live in a root-owned directory
    // (/usr/local/bin) which we cannot write to directly.
    let tmp_path = std::env::temp_dir().join("clipdeck.update_tmp");

    // ── Download ──────────────────────────────────────────────────────────────
    on_progress(format!("Downloading {}…", info.tag));
    log::info!("Downloading {} from {}", info.tag, info.download_url);

    let client = reqwest::Client::builder()
        .user_agent(concat!("clipdeck/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let response = client
        .get(&info.download_url)
        .send()
        .await?
        .error_for_status()?;

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut tmp_file = tokio::fs::File::create(&tmp_path).await?;
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt as _;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        tmp_file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if total > 0 {
            let pct = downloaded * 100 / total;
            on_progress(format!("Downloading {}… {}%", info.tag, pct));
        }
    }

    tmp_file.flush().await?;
    drop(tmp_file);

    // ── Set executable permissions ────────────────────────────────────────────
    let mut perms = std::fs::metadata(&tmp_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&tmp_path, perms)?;

    // ── Atomic replace ────────────────────────────────────────────────────────
    on_progress("Installing…".to_string());
    log::info!("Replacing {:?} with new binary", current_exe);

    // Try a direct copy first (works when the exe is user-writable).
    // Fall back to `pkexec cp` for system directories like /usr/local/bin.
    let copy_result = std::fs::copy(&tmp_path, &current_exe);
    match copy_result {
        Ok(_) => {
            std::fs::remove_file(&tmp_path).ok();
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            on_progress("Installing… (enter your password if prompted)".to_string());
            log::info!("Direct copy failed, retrying with pkexec");
            let status = std::process::Command::new("pkexec")
                .args([
                    "cp",
                    "--",
                    tmp_path.to_str().unwrap_or_default(),
                    current_exe.to_str().unwrap_or_default(),
                ])
                .status()
                .map_err(|e| anyhow::anyhow!("pkexec not found: {e}"))?;
            std::fs::remove_file(&tmp_path).ok();
            if !status.success() {
                anyhow::bail!("pkexec cp failed — update cancelled");
            }
        }
        Err(e) => return Err(e.into()),
    }

    // ── Restart ───────────────────────────────────────────────────────────────
    on_progress("Restarting…".to_string());
    restart_process(&current_exe);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Try `systemctl --user restart clipdeck.service`, fall back to exec.
/// This function does not return.
fn restart_process(exe: &std::path::Path) -> ! {
    log::info!("Attempting systemd restart…");

    let systemd_ok = std::process::Command::new("systemctl")
        .args(["--user", "restart", "clipdeck.service"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if systemd_ok {
        log::info!("systemd restart issued — exiting current process");
        std::process::exit(0);
    }

    // systemd not available or service not installed — exec the new binary
    // in-place, which replaces the current process image.
    log::info!("Exec-restarting {:?}", exe);

    use std::os::unix::process::CommandExt as _;
    let err = std::process::Command::new(exe).exec();
    // exec() only returns on failure
    log::error!("exec restart failed: {err}");
    std::process::exit(1);
}

/// Semver comparison — returns `true` if `remote` is strictly newer than `local`.
fn is_newer(remote: &str, local: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.').filter_map(|p| p.parse().ok()).collect()
    };

    let r = parse(remote);
    let l = parse(local);

    for (rv, lv) in r.iter().zip(l.iter()) {
        if rv > lv {
            return true;
        }
        if rv < lv {
            return false;
        }
    }

    r.len() > l.len()
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn newer_minor() {
        assert!(is_newer("0.2.0", "0.1.0"));
    }

    #[test]
    fn same_version() {
        assert!(!is_newer("0.1.0", "0.1.0"));
    }

    #[test]
    fn older_version() {
        assert!(!is_newer("0.0.9", "0.1.0"));
    }
}
