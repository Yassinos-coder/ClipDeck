//! GitHub release version checker.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
}

/// Check GitHub releases and return the latest tag if it is newer than
/// `current_version`.  Returns `Ok(None)` when already up to date.
pub async fn check_for_update(
    github_repo: &str,
    current_version: &str,
) -> anyhow::Result<Option<String>> {
    let url = format!(
        "https://api.github.com/repos/{github_repo}/releases/latest"
    );

    let client = reqwest::Client::builder()
        .user_agent(concat!("clipdeck/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let resp = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json::<GithubRelease>()
        .await?;

    let remote = resp.tag_name.trim_start_matches('v');
    let local = current_version.trim_start_matches('v');

    if is_newer(remote, local) {
        Ok(Some(resp.tag_name))
    } else {
        Ok(None)
    }
}

/// Very simple semver comparison: split on `.`, compare each numeric segment.
fn is_newer(remote: &str, local: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
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
