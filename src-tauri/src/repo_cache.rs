use crate::models::{AppError, RepoRef, SkillRepo};
use crate::remote_head;
use crate::skill_downloader;
use crate::skill_storage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepoCacheMeta {
    pub repo_slug: String,
    pub repo_host: String,
    pub project_path: String,
    pub branch: String,
    pub commit_sha: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheDecision {
    UseCache,
    Refresh,
    UseStaleOnApiFailure,
}

pub fn cache_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("repo-cache")
}

pub fn cache_dir(app_data_dir: &Path, repo: &SkillRepo) -> PathBuf {
    let project_path = if repo.project_path.is_empty() {
        format!("{}/{}", repo.owner, repo.name)
    } else {
        repo.project_path.clone()
    };
    let slug = skill_storage::compute_repo_slug(&repo.host, &project_path);
    cache_root(app_data_dir).join(slug).join(&repo.branch)
}

pub fn ensure_repo_tree(
    repo: &SkillRepo,
    app_data_dir: &Path,
    force: bool,
) -> Result<PathBuf, AppError> {
    ensure_repo_tree_with_hooks(
        repo,
        app_data_dir,
        force,
        remote_head::fetch_remote_head_sha,
        skill_downloader::download_repo_ref_to_dir,
    )
}

pub(crate) fn ensure_repo_tree_with_hooks<F, G>(
    repo: &SkillRepo,
    app_data_dir: &Path,
    force: bool,
    fetch_remote_sha: F,
    download_repo: G,
) -> Result<PathBuf, AppError>
where
    F: Fn(&RepoRef) -> Result<String, AppError>,
    G: Fn(&Path, &RepoRef) -> Result<(), AppError>,
{
    let cache_dir = cache_dir(app_data_dir, repo);
    let tree_dir = cache_dir.join("tree");
    let meta_path = cache_dir.join("meta.json");
    let repo_ref = repo.to_repo_ref();
    let tree_exists = tree_dir.is_dir();

    // First pull: skip commits API (avoids rate limit); download by branch, then best-effort SHA.
    if !tree_exists {
        return refresh_repo_tree(
            repo,
            &cache_dir,
            &tree_dir,
            &meta_path,
            &repo_ref,
            ShaForMeta::BestEffort,
            &fetch_remote_sha,
            &download_repo,
        );
    }

    let remote_sha = fetch_remote_sha(&repo_ref);
    let meta = read_meta(&meta_path).ok();

    match decide_cache_action(meta.as_ref(), remote_sha.as_deref(), tree_exists, force) {
        CacheDecision::UseCache => return Ok(tree_dir),
        CacheDecision::UseStaleOnApiFailure => return Ok(tree_dir),
        CacheDecision::Refresh => {}
    }

    let sha_for_meta = match remote_sha {
        Ok(sha) => ShaForMeta::Known(sha),
        // Already failed; do not burn another rate-limit call after download.
        Err(_) => ShaForMeta::Skip,
    };
    refresh_repo_tree(
        repo,
        &cache_dir,
        &tree_dir,
        &meta_path,
        &repo_ref,
        sha_for_meta,
        &fetch_remote_sha,
        &download_repo,
    )
}

enum ShaForMeta {
    Known(String),
    BestEffort,
    Skip,
}

fn refresh_repo_tree<F, G>(
    repo: &SkillRepo,
    cache_dir: &Path,
    tree_dir: &Path,
    meta_path: &Path,
    repo_ref: &RepoRef,
    sha_for_meta: ShaForMeta,
    fetch_remote_sha: &F,
    download_repo: &G,
) -> Result<PathBuf, AppError>
where
    F: Fn(&RepoRef) -> Result<String, AppError>,
    G: Fn(&Path, &RepoRef) -> Result<(), AppError>,
{
    let tree_tmp = cache_dir.join("tree.tmp");
    if tree_tmp.exists() {
        fs::remove_dir_all(&tree_tmp)
            .map_err(|err| io_error(Some(&tree_tmp), err.to_string()))?;
    }
    download_repo(&tree_tmp, repo_ref)?;

    let commit_sha = match sha_for_meta {
        ShaForMeta::Known(sha) => sha,
        ShaForMeta::BestEffort => fetch_remote_sha(repo_ref).unwrap_or_default(),
        ShaForMeta::Skip => String::new(),
    };

    let project_path = if repo.project_path.is_empty() {
        format!("{}/{}", repo.owner, repo.name)
    } else {
        repo.project_path.clone()
    };

    let new_meta = RepoCacheMeta {
        repo_slug: skill_storage::compute_repo_slug(&repo.host, &project_path),
        repo_host: repo.host.clone(),
        project_path,
        branch: repo.branch.clone(),
        commit_sha,
        fetched_at: chrono::Utc::now().to_rfc3339(),
    };

    write_meta_atomic(meta_path, &new_meta)?;
    replace_tree_dir(tree_dir, &tree_tmp)?;

    Ok(tree_dir.to_path_buf())
}

pub(crate) fn decide_cache_action(
    meta: Option<&RepoCacheMeta>,
    remote_sha: Result<&str, &AppError>,
    tree_exists: bool,
    force: bool,
) -> CacheDecision {
    if let Ok(sha) = remote_sha {
        if let Some(meta) = meta {
            if tree_exists && !meta.commit_sha.is_empty() && meta.commit_sha == sha {
                return CacheDecision::UseCache;
            }
        }
        return CacheDecision::Refresh;
    }

    if force {
        return CacheDecision::Refresh;
    }

    // Prefer existing tree when commits API fails (rate limit / offline), even past TTL.
    if tree_exists {
        return CacheDecision::UseStaleOnApiFailure;
    }

    CacheDecision::Refresh
}

fn read_meta(path: &Path) -> Result<RepoCacheMeta, AppError> {
    let raw = fs::read_to_string(path).map_err(|err| io_error(Some(path), err.to_string()))?;
    serde_json::from_str(&raw).map_err(|err| {
        io_error(
            Some(path),
            format!("无法解析 repo-cache meta.json: {}", err),
        )
    })
}

fn write_meta_atomic(path: &Path, meta: &RepoCacheMeta) -> Result<(), AppError> {
    let tmp = path.with_extension("json.tmp");
    let raw = serde_json::to_string_pretty(meta).map_err(|err| {
        io_error(
            Some(path),
            format!("无法序列化 repo-cache meta.json: {}", err),
        )
    })?;
    fs::write(&tmp, raw).map_err(|err| io_error(Some(&tmp), err.to_string()))?;
    if path.exists() {
        fs::remove_file(path).map_err(|err| io_error(Some(path), err.to_string()))?;
    }
    fs::rename(&tmp, path).map_err(|err| io_error(Some(path), err.to_string()))?;
    Ok(())
}

fn replace_tree_dir(tree_dir: &Path, tree_tmp: &Path) -> Result<(), AppError> {
    if tree_dir.exists() {
        fs::remove_dir_all(tree_dir).map_err(|err| io_error(Some(tree_dir), err.to_string()))?;
    }
    if let Some(parent) = tree_dir.parent() {
        fs::create_dir_all(parent).map_err(|err| io_error(Some(parent), err.to_string()))?;
    }
    fs::rename(tree_tmp, tree_dir).map_err(|err| io_error(Some(tree_dir), err.to_string()))?;
    Ok(())
}

fn io_error(path: Option<&Path>, message: String) -> AppError {
    AppError::Io {
        path: path.map(|value| value.to_path_buf()),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SkillRepo;

    fn sample_meta(fetched_at: &str, commit_sha: &str) -> RepoCacheMeta {
        RepoCacheMeta {
            repo_slug: "github-com--obra-superpowers".to_string(),
            repo_host: "github.com".to_string(),
            project_path: "obra/superpowers".to_string(),
            branch: "main".to_string(),
            commit_sha: commit_sha.to_string(),
            fetched_at: fetched_at.to_string(),
        }
    }

    #[test]
    fn cache_dir_uses_repo_slug_and_branch() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = SkillRepo {
            host: "github.com".to_string(),
            provider: "github".to_string(),
            project_path: "obra/superpowers".to_string(),
            owner: "obra".to_string(),
            name: "superpowers".to_string(),
            branch: "main".to_string(),
            enabled: true,
        };

        let dir = cache_dir(temp.path(), &repo);
        assert!(dir.ends_with("main"));
        assert!(dir.to_string_lossy().contains("github.com--obra-superpowers"));
    }

    #[test]
    fn decide_cache_action_hits_when_sha_matches() {
        let meta = sample_meta("2026-07-08T09:00:00Z", "abc");
        assert_eq!(
            decide_cache_action(Some(&meta), Ok("abc"), true, false),
            CacheDecision::UseCache
        );
    }

    #[test]
    fn decide_cache_action_refreshes_when_sha_differs() {
        let meta = sample_meta("2026-07-08T09:00:00Z", "old");
        assert_eq!(
            decide_cache_action(Some(&meta), Ok("new"), true, false),
            CacheDecision::Refresh
        );
    }

    #[test]
    fn decide_cache_action_uses_stale_on_api_failure_within_ttl() {
        let fetched_at = chrono::Utc::now().to_rfc3339();
        let meta = sample_meta(&fetched_at, "abc");
        let api_err = AppError::Io {
            path: None,
            message: "offline".to_string(),
        };
        assert_eq!(
            decide_cache_action(Some(&meta), Err(&api_err), true, false),
            CacheDecision::UseStaleOnApiFailure
        );
    }

    #[test]
    fn decide_cache_action_uses_stale_on_api_failure_past_ttl() {
        let meta = sample_meta("2020-01-01T00:00:00Z", "abc");
        let api_err = AppError::DownloadFailed {
            url: "https://api.github.com/repos/o/n/commits/main".to_string(),
            status: Some(403),
            message: "GitHub 请求受限，请稍后再试".to_string(),
        };
        assert_eq!(
            decide_cache_action(Some(&meta), Err(&api_err), true, false),
            CacheDecision::UseStaleOnApiFailure
        );
    }

    #[test]
    fn decide_cache_action_force_refresh_on_api_failure() {
        let fetched_at = chrono::Utc::now().to_rfc3339();
        let meta = sample_meta(&fetched_at, "abc");
        let api_err = AppError::Io {
            path: None,
            message: "offline".to_string(),
        };
        assert_eq!(
            decide_cache_action(Some(&meta), Err(&api_err), true, true),
            CacheDecision::Refresh
        );
    }

    #[test]
    fn decide_cache_action_refreshes_when_cached_sha_empty() {
        let meta = sample_meta("2026-07-08T09:00:00Z", "");
        assert_eq!(
            decide_cache_action(Some(&meta), Ok("abc"), true, false),
            CacheDecision::Refresh
        );
    }

    fn sample_repo() -> SkillRepo {
        SkillRepo {
            host: "github.com".to_string(),
            provider: "github".to_string(),
            project_path: "anthropics/skills".to_string(),
            owner: "anthropics".to_string(),
            name: "skills".to_string(),
            branch: "main".to_string(),
            enabled: true,
        }
    }

    fn write_cached_skill_tree(tree_dir: &Path) {
        let skill_dir = tree_dir.join("skills").join("writing-plans");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: writing-plans\ndescription: Create plans.\n---\n",
        )
        .expect("write skill md");
    }

    #[test]
    fn ensure_repo_tree_reuses_cache_on_second_call_when_sha_matches() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let temp = tempfile::tempdir().expect("tempdir");
        let app_data = temp.path().join("app-data");
        let repo = sample_repo();
        let download_count = AtomicUsize::new(0);
        const SHA: &str = "abc123deadbeef";

        let fetch_sha = |_repo_ref: &RepoRef| Ok(SHA.to_string());
        let download_repo = |dest_dir: &Path, _repo_ref: &RepoRef| {
            download_count.fetch_add(1, Ordering::SeqCst);
            write_cached_skill_tree(dest_dir);
            Ok(())
        };

        let tree_first = ensure_repo_tree_with_hooks(
            &repo,
            &app_data,
            false,
            fetch_sha,
            download_repo,
        )
        .expect("first ensure");
        assert_eq!(download_count.load(Ordering::SeqCst), 1);
        assert!(tree_first.join("skills/writing-plans/SKILL.md").is_file());

        let tree_second = ensure_repo_tree_with_hooks(
            &repo,
            &app_data,
            false,
            fetch_sha,
            download_repo,
        )
        .expect("second ensure");
        assert_eq!(download_count.load(Ordering::SeqCst), 1);
        assert_eq!(tree_first, tree_second);
        assert!(cache_dir(&app_data, &repo).join("meta.json").is_file());
    }

    #[test]
    fn ensure_repo_tree_redownloads_when_remote_sha_changes() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let temp = tempfile::tempdir().expect("tempdir");
        let app_data = temp.path().join("app-data");
        let repo = sample_repo();
        let download_count = AtomicUsize::new(0);
        let remote_sha = std::sync::Mutex::new("sha-v1".to_string());

        let fetch_sha = |_repo_ref: &RepoRef| Ok(remote_sha.lock().expect("lock").clone());
        let download_repo = |dest_dir: &Path, _repo_ref: &RepoRef| {
            download_count.fetch_add(1, Ordering::SeqCst);
            write_cached_skill_tree(dest_dir);
            Ok(())
        };

        ensure_repo_tree_with_hooks(&repo, &app_data, false, fetch_sha, download_repo)
            .expect("seed cache");
        assert_eq!(download_count.load(Ordering::SeqCst), 1);

        *remote_sha.lock().expect("lock") = "sha-v2".to_string();
        ensure_repo_tree_with_hooks(&repo, &app_data, false, fetch_sha, download_repo)
            .expect("refresh after sha change");
        assert_eq!(download_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn ensure_repo_tree_force_still_hits_cache_when_sha_unchanged() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let temp = tempfile::tempdir().expect("tempdir");
        let app_data = temp.path().join("app-data");
        let repo = sample_repo();
        let download_count = AtomicUsize::new(0);
        const SHA: &str = "abc123deadbeef";

        let fetch_sha = |_repo_ref: &RepoRef| Ok(SHA.to_string());
        let download_repo = |dest_dir: &Path, _repo_ref: &RepoRef| {
            download_count.fetch_add(1, Ordering::SeqCst);
            write_cached_skill_tree(dest_dir);
            Ok(())
        };

        ensure_repo_tree_with_hooks(&repo, &app_data, false, fetch_sha, download_repo)
            .expect("seed cache");
        ensure_repo_tree_with_hooks(&repo, &app_data, true, fetch_sha, download_repo)
            .expect("force discover with unchanged sha");
        assert_eq!(download_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn ensure_repo_tree_first_pull_skips_sha_and_succeeds_when_sha_api_fails() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let temp = tempfile::tempdir().expect("tempdir");
        let app_data = temp.path().join("app-data");
        let repo = sample_repo();
        let download_count = AtomicUsize::new(0);
        let sha_count = AtomicUsize::new(0);

        let fetch_sha = |_repo_ref: &RepoRef| {
            sha_count.fetch_add(1, Ordering::SeqCst);
            Err(AppError::DownloadFailed {
                url: "https://api.github.com/repos/anthropics/skills/commits/main".to_string(),
                status: Some(403),
                message: "GitHub 请求受限，请稍后再试".to_string(),
            })
        };
        let download_repo = |dest_dir: &Path, _repo_ref: &RepoRef| {
            download_count.fetch_add(1, Ordering::SeqCst);
            write_cached_skill_tree(dest_dir);
            Ok(())
        };

        let tree = ensure_repo_tree_with_hooks(&repo, &app_data, false, fetch_sha, download_repo)
            .expect("first pull should download without requiring SHA");
        assert_eq!(download_count.load(Ordering::SeqCst), 1);
        assert!(tree.join("skills/writing-plans/SKILL.md").is_file());

        let meta = read_meta(&cache_dir(&app_data, &repo).join("meta.json")).expect("meta");
        assert!(meta.commit_sha.is_empty());
        // Best-effort SHA after download still attempted once.
        assert_eq!(sha_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn ensure_repo_tree_uses_stale_when_sha_api_fails_after_cache() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let temp = tempfile::tempdir().expect("tempdir");
        let app_data = temp.path().join("app-data");
        let repo = sample_repo();
        let download_count = AtomicUsize::new(0);
        let allow_sha = std::sync::atomic::AtomicBool::new(true);

        let fetch_sha = |_repo_ref: &RepoRef| {
            if allow_sha.load(Ordering::SeqCst) {
                Ok("abc123deadbeef".to_string())
            } else {
                Err(AppError::DownloadFailed {
                    url: "https://api.github.com/repos/anthropics/skills/commits/main".to_string(),
                    status: Some(403),
                    message: "GitHub 请求受限，请稍后再试".to_string(),
                })
            }
        };
        let download_repo = |dest_dir: &Path, _repo_ref: &RepoRef| {
            download_count.fetch_add(1, Ordering::SeqCst);
            write_cached_skill_tree(dest_dir);
            Ok(())
        };

        ensure_repo_tree_with_hooks(&repo, &app_data, false, fetch_sha, download_repo)
            .expect("seed");
        assert_eq!(download_count.load(Ordering::SeqCst), 1);

        allow_sha.store(false, Ordering::SeqCst);
        ensure_repo_tree_with_hooks(&repo, &app_data, false, fetch_sha, download_repo)
            .expect("stale cache on 403");
        assert_eq!(download_count.load(Ordering::SeqCst), 1);
    }
}
