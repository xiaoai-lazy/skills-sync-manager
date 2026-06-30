use crate::models::{
    AppConfig, AppError, RepoRef, SkillRecord, SkillUpdateInfo, UpdateAllSkillsFailure,
    UpdateAllSkillsResult,
};
use crate::skill_discover::iso8601_timestamp_now;
use crate::skill_downloader::{self, copy_dir_recursive};
use crate::skill_install::resolve_skill_directory;
use crate::skill_repos;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn compute_dir_hash(dir: &Path) -> Result<String, AppError> {
    let mut files = Vec::new();
    collect_files_sorted(dir, dir, &mut files)?;
    files.sort();

    let mut hasher = Sha256::new();
    for relative_path in files {
        hasher.update(relative_path.as_bytes());
        let file_path = dir.join(&relative_path);
        let mut file =
            fs::File::open(&file_path).map_err(|err| io_error(Some(&file_path), err.to_string()))?;
        let mut buffer = [0u8; 8192];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|err| io_error(Some(&file_path), err.to_string()))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_files_sorted(
    root: &Path,
    current: &Path,
    files: &mut Vec<String>,
) -> Result<(), AppError> {
    if current.is_file() {
        let relative = current
            .strip_prefix(root)
            .map_err(|_| AppError::Io {
                path: Some(current.to_path_buf()),
                message: "无法计算相对路径".to_string(),
            })?
            .to_string_lossy()
            .replace('\\', "/");
        files.push(relative);
        return Ok(());
    }

    if !current.is_dir() {
        return Ok(());
    }

    let mut entries = fs::read_dir(current)
        .map_err(|err| io_error(Some(current), err.to_string()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        collect_files_sorted(root, &path, files)?;
    }

    Ok(())
}

pub fn check_updates(config: &AppConfig, main_dir: &Path) -> Result<Vec<SkillUpdateInfo>, AppError> {
    check_updates_with_download_hook(config, main_dir, |repo_ref| {
        skill_downloader::download_repo_ref(repo_ref)
    })
}

pub fn check_updates_with_download_hook<F>(
    config: &AppConfig,
    main_dir: &Path,
    download_repo_ref: F,
) -> Result<Vec<SkillUpdateInfo>, AppError>
where
    F: Fn(&RepoRef) -> Result<PathBuf, AppError>,
{
    let mut repo_cache: HashMap<RepoCacheKey, PathBuf> = HashMap::new();
    let mut updates = Vec::new();

    for (dir_name, record) in &config.skill_records {
        if !skill_repos::is_skill_repo_enabled(config, &record.repo_host, &record.project_path) {
            continue;
        }
        let update = check_single_record(
            dir_name,
            record,
            main_dir,
            &mut repo_cache,
            &download_repo_ref,
        )?;
        if let Some(info) = update {
            updates.push(info);
        }
    }

    updates.sort_by(|left, right| left.dir_name.cmp(&right.dir_name));
    Ok(updates)
}

type RepoCacheKey = (String, String, String);

fn check_single_record<F>(
    dir_name: &str,
    record: &SkillRecord,
    main_dir: &Path,
    repo_cache: &mut HashMap<RepoCacheKey, PathBuf>,
    download_repo_ref: &F,
) -> Result<Option<SkillUpdateInfo>, AppError>
where
    F: Fn(&RepoRef) -> Result<PathBuf, AppError>,
{
    let repo_ref = record.to_repo_ref();
    let repo_key = (
        repo_ref.host.clone(),
        repo_ref.project_path.clone(),
        repo_ref.branch.clone(),
    );
    let repo_root = if let Some(cached) = repo_cache.get(&repo_key) {
        cached.clone()
    } else {
        let downloaded = download_repo_ref(&repo_ref)?;
        repo_cache.insert(repo_key, downloaded.clone());
        downloaded
    };

    let remote_dir = resolve_skill_directory(&repo_root, &record.directory);
    if !remote_dir.join("SKILL.md").is_file() {
        return Err(AppError::SkillDirNotFound {
            path: remote_dir,
        });
    }

    let remote_hash = compute_dir_hash(&remote_dir)?;
    let local_path = main_dir.join(dir_name);
    let current_hash = if local_path.is_dir() {
        Some(compute_dir_hash(&local_path)?)
    } else {
        None
    };

    if current_hash.as_deref() == Some(remote_hash.as_str()) {
        return Ok(None);
    }

    Ok(Some(SkillUpdateInfo {
        dir_name: dir_name.to_string(),
        name: skill_display_name(&local_path, dir_name),
        current_hash,
        remote_hash,
    }))
}

fn skill_display_name(skill_dir: &Path, fallback: &str) -> String {
    let skill_md = skill_dir.join("SKILL.md");
    if skill_md.is_file() {
        if let Ok(raw) = fs::read_to_string(&skill_md) {
            if let Some(metadata) = crate::skill_library::parse_valid_skill_metadata(&raw) {
                return metadata.name;
            }
        }
    }
    fallback.to_string()
}

pub fn update_skill(
    config: &mut AppConfig,
    dir_name: &str,
    main_dir: &Path,
) -> Result<(), AppError> {
    update_skill_with_download_hook(config, dir_name, main_dir, |repo_ref| {
        skill_downloader::download_repo_ref(repo_ref)
    })
}

pub fn update_skill_with_download_hook<F>(
    config: &mut AppConfig,
    dir_name: &str,
    main_dir: &Path,
    download_repo_ref: F,
) -> Result<(), AppError>
where
    F: FnOnce(&RepoRef) -> Result<PathBuf, AppError>,
{
    if !config
        .skill_update_cache
        .updates
        .iter()
        .any(|update| update.dir_name == dir_name)
    {
        return Err(AppError::UpdateNotPending {
            dir_name: dir_name.to_string(),
        });
    }

    let record = config
        .skill_records
        .get(dir_name)
        .cloned()
        .ok_or_else(|| AppError::UpdateNotPending {
            dir_name: dir_name.to_string(),
        })?;

    let repo_ref = record.to_repo_ref();
    let repo_root = download_repo_ref(&repo_ref)?;
    let source_dir = resolve_skill_directory(&repo_root, &record.directory);

    if !source_dir.join("SKILL.md").is_file() {
        return Err(AppError::SkillDirNotFound {
            path: source_dir,
        });
    }

    let install_path = main_dir.join(dir_name);
    if install_path.exists() {
        ensure_deletable_skill_dir(&install_path, dir_name)?;
        crate::fs_adapter::delete_real_dir(&install_path)?;
    }

    copy_dir_recursive(&source_dir, &install_path)?;
    let content_hash = compute_dir_hash(&install_path)?;

    if let Some(record) = config.skill_records.get_mut(dir_name) {
        record.content_hash = content_hash;
    }

    config
        .skill_update_cache
        .updates
        .retain(|update| update.dir_name != dir_name);

    Ok(())
}

pub fn update_all_skills(
    config: &mut AppConfig,
    main_dir: &Path,
) -> Result<UpdateAllSkillsResult, AppError> {
    update_all_skills_with_download_hook(config, main_dir, |repo_ref| {
        skill_downloader::download_repo_ref(repo_ref)
    })
}

pub fn update_all_skills_with_download_hook<F>(
    config: &mut AppConfig,
    main_dir: &Path,
    mut download_repo_ref: F,
) -> Result<UpdateAllSkillsResult, AppError>
where
    F: FnMut(&RepoRef) -> Result<PathBuf, AppError>,
{
    let pending: Vec<String> = config
        .skill_update_cache
        .updates
        .iter()
        .map(|update| update.dir_name.clone())
        .collect();

    let mut updated = Vec::new();
    let mut failed = Vec::new();

    for dir_name in pending {
        match update_skill_with_download_hook(config, &dir_name, main_dir, |repo_ref| {
            download_repo_ref(repo_ref)
        }) {
            Ok(()) => updated.push(dir_name),
            Err(err) => failed.push(UpdateAllSkillsFailure {
                dir_name,
                error: err.to_string(),
            }),
        }
    }

    if failed.is_empty() {
        config.skill_update_cache.updates.clear();
    }

    updated.sort();
    failed.sort_by(|left, right| left.dir_name.cmp(&right.dir_name));

    Ok(UpdateAllSkillsResult { updated, failed })
}

pub fn apply_check_updates_cache(config: &mut AppConfig, updates: Vec<SkillUpdateInfo>) {
    config.skill_update_cache = crate::models::SkillUpdateCache {
        checked_at: Some(iso8601_timestamp_now()),
        updates,
    };
}

fn ensure_deletable_skill_dir(path: &Path, dir_name: &str) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(AppError::Io {
                path: Some(path.to_path_buf()),
                message: format!("源 skill 路径是链接，无法更新：{}", dir_name),
            });
        }
        _ => {}
    }
    Ok(())
}

fn io_error(path: Option<impl AsRef<Path>>, message: String) -> AppError {
    AppError::Io {
        path: path.map(|value| value.as_ref().to_path_buf()),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SkillDiscoverCache, SkillRepo, SkillUpdateCache, default_github_host};
    use std::fs;

    fn write_valid_skill(dir: &Path, name: &str, body_suffix: &str) {
        fs::create_dir_all(dir).expect("create skill dir");
        fs::write(
            dir.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: Test skill.\n---\n\n# Skill {body_suffix}\n",
                name
            ),
        )
        .expect("write skill md");
    }

    fn sample_record(directory: &str, content_hash: &str) -> SkillRecord {
        SkillRecord {
            repo_host: default_github_host(),
            project_path: "anthropics/skills".to_string(),
            source: "github".to_string(),
            repo_owner: "anthropics".to_string(),
            repo_name: "skills".to_string(),
            repo_branch: "main".to_string(),
            directory: directory.to_string(),
            content_hash: content_hash.to_string(),
            installed_at: "2026-06-30T00:00:00Z".to_string(),
        }
    }

    fn config_with_record(dir_name: &str, record: SkillRecord) -> AppConfig {
        let mut config = AppConfig::default();
        config.skill_records.insert(dir_name.to_string(), record);
        config
    }

    #[test]
    fn compute_dir_hash_is_stable_for_same_contents() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_skill(&temp.path().join("skill"), "example", "a");
        fs::write(temp.path().join("skill").join("extra.txt"), "payload").expect("write extra");

        let first = compute_dir_hash(&temp.path().join("skill")).expect("hash first");
        let second = compute_dir_hash(&temp.path().join("skill")).expect("hash second");

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn check_updates_skips_disabled_source_repo() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        let repo_root = temp.path().join("repo");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let local_installed = main_dir.join("brainstorming");
        write_valid_skill(&local_installed, "brainstorming", "local-old");

        write_valid_skill(
            &repo_root.join("skills").join("brainstorming"),
            "brainstorming",
            "remote-new",
        );

        let mut config = config_with_record(
            "brainstorming",
            sample_record("skills/brainstorming", "stale-hash"),
        );
        config.skill_repos = vec![SkillRepo {
            host: default_github_host(),
            provider: "github".to_string(),
            project_path: "anthropics/skills".to_string(),
            owner: "anthropics".to_string(),
            name: "skills".to_string(),
            branch: "main".to_string(),
            enabled: false,
        }];

        let updates = check_updates_with_download_hook(&config, &main_dir, |_| {
            Ok(repo_root.clone())
        })
        .expect("check updates");

        assert!(updates.is_empty());
    }

    #[test]
    fn check_updates_skills_without_skill_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        let repo_root = temp.path().join("repo");
        fs::create_dir_all(&main_dir).expect("create main dir");
        write_valid_skill(&main_dir.join("legacy-skill"), "legacy-skill", "v1");
        write_valid_skill(
            &repo_root.join("skills").join("brainstorming"),
            "brainstorming",
            "remote",
        );

        let local_installed = main_dir.join("brainstorming");
        write_valid_skill(&local_installed, "brainstorming", "local-old");

        let config = config_with_record(
            "brainstorming",
            sample_record("skills/brainstorming", "stale-hash"),
        );

        let updates = check_updates_with_download_hook(&config, &main_dir, |_| {
            Ok(repo_root.clone())
        })
        .expect("check updates");

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].dir_name, "brainstorming");
        assert!(config.skill_records.get("legacy-skill").is_none());
        assert!(updates.iter().all(|update| update.dir_name != "legacy-skill"));
    }

    #[test]
    fn check_updates_detects_hash_change_with_local_fixture() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        let repo_root = temp.path().join("repo");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let local_installed = main_dir.join("brainstorming");
        write_valid_skill(&local_installed, "brainstorming", "local-old");
        let local_hash = compute_dir_hash(&local_installed).expect("local hash");

        write_valid_skill(
            &repo_root.join("skills").join("brainstorming"),
            "brainstorming",
            "remote-new",
        );
        let remote_hash = compute_dir_hash(&repo_root.join("skills").join("brainstorming"))
            .expect("remote hash");
        assert_ne!(local_hash, remote_hash);

        let config = config_with_record(
            "brainstorming",
            sample_record("skills/brainstorming", &local_hash),
        );

        let updates =
            check_updates_with_download_hook(&config, &main_dir, |_| Ok(repo_root.clone()))
                .expect("check updates");

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].dir_name, "brainstorming");
        assert_eq!(updates[0].current_hash.as_deref(), Some(local_hash.as_str()));
        assert_eq!(updates[0].remote_hash, remote_hash);
    }

    #[test]
    fn update_skill_overwrites_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        let repo_root = temp.path().join("repo");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let install_path = main_dir.join("brainstorming");
        write_valid_skill(&install_path, "brainstorming", "local-old");
        fs::write(install_path.join("stale.txt"), "old").expect("write stale");

        write_valid_skill(
            &repo_root.join("skills").join("brainstorming"),
            "brainstorming",
            "remote-new",
        );
        fs::write(
            repo_root
                .join("skills")
                .join("brainstorming")
                .join("fresh.txt"),
            "new",
        )
        .expect("write fresh");

        let remote_hash =
            compute_dir_hash(&repo_root.join("skills").join("brainstorming")).expect("remote hash");

        let mut config = config_with_record(
            "brainstorming",
            sample_record("skills/brainstorming", "stale-hash"),
        );
        config.skill_update_cache = SkillUpdateCache {
            checked_at: Some("2026-06-30T00:00:00Z".to_string()),
            updates: vec![SkillUpdateInfo {
                dir_name: "brainstorming".to_string(),
                name: "brainstorming".to_string(),
                current_hash: Some("stale-hash".to_string()),
                remote_hash: remote_hash.clone(),
            }],
        };

        update_skill_with_download_hook(&mut config, "brainstorming", &main_dir, |_| {
            Ok(repo_root.clone())
        })
        .expect("update skill");

        assert!(!install_path.join("stale.txt").exists());
        assert_eq!(fs::read_to_string(install_path.join("fresh.txt")).unwrap(), "new");
        assert_eq!(
            config
                .skill_records
                .get("brainstorming")
                .expect("record")
                .content_hash,
            remote_hash
        );
        assert!(config.skill_update_cache.updates.is_empty());
    }

    #[test]
    fn update_skill_requires_pending_cache_entry() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let mut config = config_with_record(
            "brainstorming",
            sample_record("skills/brainstorming", "hash"),
        );

        let error = update_skill_with_download_hook(&mut config, "brainstorming", &main_dir, |_| {
            Ok(PathBuf::new())
        })
        .expect_err("should require pending update");

        assert!(matches!(error, AppError::UpdateNotPending { .. }));
    }

    #[test]
    fn update_all_skills_reports_partial_failures() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        let repo_root = temp.path().join("repo");
        fs::create_dir_all(&main_dir).expect("create main dir");

        write_valid_skill(&main_dir.join("good"), "good", "local");
        write_valid_skill(&repo_root.join("skills").join("good"), "good", "remote");
        write_valid_skill(&main_dir.join("bad"), "bad", "local");

        let good_remote_hash =
            compute_dir_hash(&repo_root.join("skills").join("good")).expect("good remote hash");

        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache::default(),
            ..Default::default()
        };
        config
            .skill_records
            .insert("good".to_string(), sample_record("skills/good", "stale"));
        config
            .skill_records
            .insert("bad".to_string(), sample_record("skills/bad", "stale"));
        config.skill_update_cache = SkillUpdateCache {
            checked_at: Some("2026-06-30T00:00:00Z".to_string()),
            updates: vec![
                SkillUpdateInfo {
                    dir_name: "good".to_string(),
                    name: "good".to_string(),
                    current_hash: Some("stale".to_string()),
                    remote_hash: good_remote_hash,
                },
                SkillUpdateInfo {
                    dir_name: "bad".to_string(),
                    name: "bad".to_string(),
                    current_hash: Some("stale".to_string()),
                    remote_hash: "deadbeef".to_string(),
                },
            ],
        };

        let result = update_all_skills_with_download_hook(&mut config, &main_dir, |_| {
            Ok(repo_root.clone())
        })
        .expect("update all");

        assert_eq!(result.updated, vec!["good".to_string()]);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].dir_name, "bad");
        assert_eq!(config.skill_update_cache.updates.len(), 1);
        assert_eq!(config.skill_update_cache.updates[0].dir_name, "bad");
    }
}
