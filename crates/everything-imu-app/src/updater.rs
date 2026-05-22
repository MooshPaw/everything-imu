//! GitHub-release auto-update.
//!
//! Queries the `matiaspalmac/everything-imu` releases feed for the newest
//! semver-tagged build, compares against `CARGO_PKG_VERSION`, and — when
//! the user accepts — downloads the platform-specific bundle and swaps
//! the running binary in place. All blocking work is done on a
//! dedicated tokio blocking thread because the underlying `self_update`
//! crate is sync-only.

use serde::{Deserialize, Serialize};
use thiserror::Error;

const GITHUB_OWNER: &str = "matiaspalmac";
const GITHUB_REPO: &str = "everything-imu";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub url: String,
    pub update_available: bool,
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("self_update: {0}")]
    Backend(#[from] self_update::errors::Error),
    #[error("join: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// Fetch the latest release tag and report whether it differs from the
/// running version. Network-only; no files are touched.
pub async fn check() -> Result<UpdateInfo, UpdateError> {
    tokio::task::spawn_blocking(|| {
        let release = self_update::backends::github::ReleaseList::configure()
            .repo_owner(GITHUB_OWNER)
            .repo_name(GITHUB_REPO)
            .build()?
            .fetch()?
            .into_iter()
            .next()
            .ok_or_else(|| self_update::errors::Error::Release("no releases found".into()))?;
        let latest = release.version.trim_start_matches('v').to_string();
        let current = CURRENT_VERSION.to_string();
        let update_available = semver_newer(&latest, &current);
        Ok(UpdateInfo {
            current,
            latest,
            url: release.name.clone(),
            update_available,
        })
    })
    .await?
}

/// Download the appropriate release asset for the current OS+arch and
/// replace the running binary atomically. Caller is expected to prompt
/// the user to restart on success.
pub async fn apply() -> Result<UpdateInfo, UpdateError> {
    tokio::task::spawn_blocking(|| {
        let status = self_update::backends::github::Update::configure()
            .repo_owner(GITHUB_OWNER)
            .repo_name(GITHUB_REPO)
            .bin_name("everything-imu")
            .show_download_progress(false)
            .show_output(false)
            .no_confirm(true)
            .current_version(CURRENT_VERSION)
            .build()?
            .update()?;
        Ok(UpdateInfo {
            current: CURRENT_VERSION.into(),
            latest: status.version().to_string(),
            url: String::new(),
            update_available: false,
        })
    })
    .await?
}

/// Compare two SemVer strings, returning true when `candidate` is strictly
/// greater than `current`. Falls back to lexicographic comparison if a
/// component is non-numeric (prerelease tags etc.).
fn semver_newer(candidate: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|p| !p.is_empty())
            .filter_map(|p| p.parse::<u32>().ok())
            .collect()
    };
    let a = parse(candidate);
    let b = parse(current);
    for (x, y) in a.iter().zip(b.iter()) {
        if x > y {
            return true;
        }
        if x < y {
            return false;
        }
    }
    a.len() > b.len()
}

#[cfg(test)]
mod tests {
    use super::semver_newer;

    #[test]
    fn newer_major() {
        assert!(semver_newer("2.0.0", "1.9.9"));
    }

    #[test]
    fn newer_patch() {
        assert!(semver_newer("1.0.0-beta.6", "1.0.0-beta.5"));
    }

    #[test]
    fn not_newer_equal() {
        assert!(!semver_newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn not_newer_older() {
        assert!(!semver_newer("1.0.0-beta.4", "1.0.0-beta.5"));
    }
}
