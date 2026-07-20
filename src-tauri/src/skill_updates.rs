use crate::models::{
    AppConfig, AppError, RepoRef, SkillRecord, SkillRepo, SkillUpdateInfo, UpdateAllSkillsFailure,
    UpdateAllSkillsResult,
};
use crate::skill_discover::iso8601_timestamp_now;
use crate::skill_downloader::{self, copy_dir_recursive};
use crate::skill_hub_client;
use crate::skill_hub_endpoints;
use crate::skill_install::resolve_skill_directory;
use crate::skill_repos;
use crate::skill_storage;
use sha2::{Digest, Sha256};
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

/// Hub 服务端对 SKILL.md 做 SHA256 后取前 12 位十六进制，与 `compute_dir_hash` 不兼容。
pub fn compute_skill_md_hash_prefix(skill_dir: &Path) -> Result<String, AppError> {
    let skill_md = skill_dir.join("SKILL.md");
    let content = fs::read_to_string(&skill_md).map_err(|err| io_error(Some(&skill_md), err.to_string()))?;
    let digest = Sha256::digest(content.as_bytes());
    let full = format!("{:x}", digest);
    Ok(full[..12.min(full.len())].to_string())
}

fn local_hash_for_hub_compare(local_path: &Path, remote_hash: &str) -> Result<String, AppError> {
    if remote_hash.len() == 12 && remote_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        compute_skill_md_hash_prefix(local_path)
    } else {
        compute_dir_hash(local_path)
    }
}

pub fn hash_matching_stored_content_hash(path: &Path, content_hash: &str) -> Result<String, AppError> {
    local_hash_for_hub_compare(path, content_hash)
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

pub fn check_updates(
    config: &mut AppConfig,
    main_dir: &Path,
) -> Result<Vec<SkillUpdateInfo>, AppError> {
    check_updates_with_hooks(
        config,
        main_dir,
        |repo_ref| skill_downloader::download_repo_ref(repo_ref),
        |base_url, group, skill_id| skill_hub_client::download_archive(base_url, group, skill_id),
    )
}

pub fn check_updates_with_download_hook<F>(
    config: &mut AppConfig,
    main_dir: &Path,
    download_repo_ref: F,
) -> Result<Vec<SkillUpdateInfo>, AppError>
where
    F: Fn(&RepoRef) -> Result<PathBuf, AppError>,
{
    check_updates_with_hooks(config, main_dir, download_repo_ref, |_, _, _| {
        Err(AppError::Io {
            path: None,
            message: "Hub 下载未配置".to_string(),
        })
    })
}

pub fn check_updates_with_hooks<F, G>(
    config: &mut AppConfig,
    main_dir: &Path,
    download_repo_ref: F,
    download_hub_archive: G,
) -> Result<Vec<SkillUpdateInfo>, AppError>
where
    F: Fn(&RepoRef) -> Result<PathBuf, AppError>,
    G: Fn(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    let mut updates = Vec::new();

    for repo in config.skill_repos.iter().filter(|repo| repo.enabled) {
        let repo_ref = repo.to_repo_ref();
        let repo_root = match download_repo_ref(&repo_ref) {
            Ok(root) => root,
            Err(_) => continue,
        };

        for (record_key, record) in &config.skill_records {
            if !skill_repos::record_belongs_to_skill_repo(record, repo) {
                continue;
            }
            if let Some(info) =
                compare_record_to_repo_root(record_key, record, main_dir, &repo_root)?
            {
                updates.push(info);
            }
        }
    }

    let hub_keys: Vec<String> = config
        .skill_records
        .iter()
        .filter(|(_, record)| {
            record.source == "skillhub"
                && !record.source_missing
                && is_hub_endpoint_enabled(config, &record.hub_endpoint_id)
        })
        .map(|(key, _)| key.clone())
        .collect();

    for record_key in hub_keys {
        let record = match config.skill_records.get(&record_key).cloned() {
            Some(record) => record,
            None => continue,
        };
        match check_hub_record(
            &record_key,
            &record,
            main_dir,
            config,
            &download_hub_archive,
        ) {
            Ok(Some(info)) => updates.push(info),
            Ok(None) => {}
            Err(AppError::HubSkillGone { .. }) => {
                mark_record_source_missing(config, &record_key);
            }
            Err(_) => {}
        }
    }

    updates.sort_by(|left, right| left.dir_name.cmp(&right.dir_name));
    Ok(updates)
}

pub fn check_repo_updates_strict(
    config: &AppConfig,
    main_dir: &Path,
    provider: &str,
) -> Result<Vec<SkillUpdateInfo>, AppError> {
    check_repo_updates_strict_with_hook(config, main_dir, provider, |repo| {
        skill_downloader::download_repo_ref(&repo.to_repo_ref())
    })
}

fn check_repo_updates_strict_with_hook<F>(
    config: &AppConfig,
    main_dir: &Path,
    provider: &str,
    mut download_repo: F,
) -> Result<Vec<SkillUpdateInfo>, AppError>
where
    F: FnMut(&SkillRepo) -> Result<PathBuf, AppError>,
{
    let mut updates = Vec::new();
    for repo in config
        .skill_repos
        .iter()
        .filter(|repo| repo.enabled && repo.provider == provider)
    {
        let repo_root = download_repo(repo)?;
        for (record_key, record) in &config.skill_records {
            if !skill_repos::record_belongs_to_skill_repo(record, repo) {
                continue;
            }
            if let Some(info) =
                compare_record_to_repo_root(record_key, record, main_dir, &repo_root)?
            {
                updates.push(info);
            }
        }
    }
    updates.sort_by(|left, right| left.dir_name.cmp(&right.dir_name));
    Ok(updates)
}

pub fn check_hub_updates_strict(
    config: &mut AppConfig,
    main_dir: &Path,
) -> Result<Vec<SkillUpdateInfo>, AppError> {
    check_hub_updates_strict_with_hook(config, main_dir, |base_url, group, skill_id| {
        skill_hub_client::download_archive(base_url, group, skill_id)
    })
}

fn check_hub_updates_strict_with_hook<G>(
    config: &mut AppConfig,
    main_dir: &Path,
    download_hub_archive: G,
) -> Result<Vec<SkillUpdateInfo>, AppError>
where
    G: Fn(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    let hub_keys = config
        .skill_records
        .iter()
        .filter(|(_, record)| {
            record.source == "skillhub"
                && !record.source_missing
                && is_hub_endpoint_enabled(config, &record.hub_endpoint_id)
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let mut updates = Vec::new();

    for record_key in hub_keys {
        let record = config
            .skill_records
            .get(&record_key)
            .cloned()
            .expect("collected record key must exist");
        if let Some(info) = check_hub_record(
            &record_key,
            &record,
            main_dir,
            config,
            &download_hub_archive,
        )? {
            updates.push(info);
        }
    }

    updates.sort_by(|left, right| left.dir_name.cmp(&right.dir_name));
    Ok(updates)
}

fn mark_record_source_missing(config: &mut AppConfig, record_key: &str) {
    if let Some(record) = config.skill_records.get_mut(record_key) {
        record.source_missing = true;
    }
    config
        .skill_update_cache
        .updates
        .retain(|update| !pending_update_matches(update, record_key));
}

fn resolve_local_library_path(main_dir: &Path, record_key: &str, record: &SkillRecord) -> PathBuf {
    if !record.storage_key.is_empty() {
        skill_storage::main_library_path(main_dir, &record.storage_key)
    } else {
        main_dir.join(record_key)
    }
}

fn record_link_name(record_key: &str, record: &SkillRecord) -> String {
    if !record.link_name.is_empty() {
        return record.link_name.clone();
    }
    if !record.directory.is_empty() {
        return skill_storage::skill_id_from_directory(&record.directory);
    }
    if record_key.contains('/') {
        skill_storage::skill_id_from_directory(record_key)
    } else {
        record_key.to_string()
    }
}

fn resolve_record_by_identifier(
    config: &AppConfig,
    identifier: &str,
) -> Option<(String, SkillRecord)> {
    if let Some(record) = config.skill_records.get(identifier) {
        return Some((identifier.to_string(), record.clone()));
    }

    config
        .skill_records
        .iter()
        .find(|(key, record)| {
            record.storage_key == identifier || key.as_str() == identifier
        })
        .map(|(key, record)| (key.clone(), record.clone()))
}

fn pending_update_matches(update: &SkillUpdateInfo, identifier: &str) -> bool {
    !update.storage_key.is_empty() && update.storage_key == identifier
}

/// Canonical install identity for update cache entries.
/// Prefer the skill_records map key (same as SkillView.storageKey) over a possibly
/// stale/empty `record.storage_key` field so UI matching stays consistent.
fn update_storage_key(record_key: &str, record: &SkillRecord) -> String {
    if !record.storage_key.is_empty() && record.storage_key == record_key {
        return record.storage_key.clone();
    }
    if !record_key.is_empty() {
        return record_key.to_string();
    }
    record.storage_key.clone()
}

fn is_hub_endpoint_enabled(config: &AppConfig, hub_endpoint_id: &str) -> bool {
    config
        .skill_hub_endpoints
        .iter()
        .find(|endpoint| endpoint.id == hub_endpoint_id)
        .map(|endpoint| endpoint.enabled)
        .unwrap_or(false)
}

fn check_hub_record<G>(
    record_key: &str,
    record: &SkillRecord,
    main_dir: &Path,
    config: &AppConfig,
    download_hub_archive: &G,
) -> Result<Option<SkillUpdateInfo>, AppError>
where
    G: Fn(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    let remote_hash = resolve_hub_remote_hash(config, record, download_hub_archive)?;
    let local_path = resolve_local_library_path(main_dir, record_key, record);
    let link_name = record_link_name(record_key, record);
    let current_hash = if local_path.is_dir() {
        Some(local_hash_for_hub_compare(&local_path, &remote_hash)?)
    } else {
        None
    };

    if current_hash.as_deref() == Some(remote_hash.as_str()) {
        return Ok(None);
    }

    Ok(Some(SkillUpdateInfo {
        dir_name: link_name.clone(),
        name: skill_display_name(&local_path, &link_name),
        current_hash,
        remote_hash,
        storage_key: update_storage_key(record_key, record),
    }))
}

fn resolve_hub_remote_hash<G>(
    config: &AppConfig,
    record: &SkillRecord,
    download_hub_archive: &G,
) -> Result<String, AppError>
where
    G: Fn(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    let base_url =
        skill_hub_endpoints::hub_endpoint_base_url(config, &record.hub_endpoint_id)?;

    if let Ok(skills) =
        skill_hub_client::fetch_skills(&base_url, Some(&record.hub_skill_group))
    {
        if let Some(dto) = skills.iter().find(|skill| {
            skill.id == record.hub_skill_id && skill.group == record.hub_skill_group
        }) {
            if let Some(hash) = dto.hash.as_ref().filter(|hash| !hash.is_empty()) {
                return Ok(hash.clone());
            }
        } else {
            return Err(AppError::HubSkillGone {
                skill_id: record.hub_skill_id.clone(),
                group: record.hub_skill_group.clone(),
            });
        }
    }

    hash_hub_archive(
        &base_url,
        &record.hub_skill_group,
        &record.hub_skill_id,
        download_hub_archive,
    )
}

fn hash_hub_archive<G>(
    base_url: &str,
    group: &str,
    skill_id: &str,
    download_hub_archive: &G,
) -> Result<String, AppError>
where
    G: Fn(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    let zip_path = download_hub_archive(base_url, group, skill_id)?;
    let temp_dir = create_temp_extract_dir()?;
    let extract_result = skill_downloader::extract_zip_file(&zip_path, &temp_dir);
    let _ = fs::remove_file(&zip_path);
    extract_result?;
    let hash = compute_dir_hash(&temp_dir)?;
    let _ = fs::remove_dir_all(&temp_dir);
    Ok(hash)
}

fn create_temp_extract_dir() -> Result<PathBuf, AppError> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|err| io_error(None::<&Path>, err.to_string()))?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("skills-sync-hub-hash-{}", nanos));
    fs::create_dir_all(&dir).map_err(|err| io_error(Some(&dir), err.to_string()))?;
    Ok(dir)
}

fn compare_record_to_repo_root(
    record_key: &str,
    record: &SkillRecord,
    main_dir: &Path,
    repo_root: &Path,
) -> Result<Option<SkillUpdateInfo>, AppError> {
    let remote_dir = resolve_skill_directory(repo_root, &record.directory);
    if !remote_dir.join("SKILL.md").is_file() {
        return Err(AppError::SkillDirNotFound {
            path: remote_dir,
        });
    }

    let remote_hash = compute_dir_hash(&remote_dir)?;
    let local_path = resolve_local_library_path(main_dir, record_key, record);
    let link_name = record_link_name(record_key, record);
    let current_hash = if local_path.is_dir() {
        Some(compute_dir_hash(&local_path)?)
    } else {
        None
    };

    if current_hash.as_deref() == Some(remote_hash.as_str()) {
        return Ok(None);
    }

    Ok(Some(SkillUpdateInfo {
        dir_name: link_name.clone(),
        name: skill_display_name(&local_path, &link_name),
        current_hash,
        remote_hash,
        storage_key: update_storage_key(record_key, record),
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
    update_skill_with_hooks(
        config,
        dir_name,
        main_dir,
        |repo_ref| skill_downloader::download_repo_ref(repo_ref),
        |base_url, group, skill_id| skill_hub_client::download_archive(base_url, group, skill_id),
    )
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
    update_skill_with_hooks(config, dir_name, main_dir, download_repo_ref, |_, _, _| {
        Err(AppError::Io {
            path: None,
            message: "Hub 下载未配置".to_string(),
        })
    })
}

pub fn update_skill_with_hooks<F, G>(
    config: &mut AppConfig,
    dir_name: &str,
    main_dir: &Path,
    download_repo_ref: F,
    download_hub_archive: G,
) -> Result<(), AppError>
where
    F: FnOnce(&RepoRef) -> Result<PathBuf, AppError>,
    G: FnOnce(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    let (record_key, record) = resolve_record_by_identifier(config, dir_name).ok_or_else(|| {
        AppError::UpdateNotPending {
            dir_name: dir_name.to_string(),
        }
    })?;
    let update_key = if !record.storage_key.is_empty() {
        record.storage_key.clone()
    } else {
        record_key.clone()
    };

    if !config
        .skill_update_cache
        .updates
        .iter()
        .any(|update| pending_update_matches(update, &update_key))
    {
        return Err(AppError::UpdateNotPending {
            dir_name: dir_name.to_string(),
        });
    }

    if record.source == "skillhub" {
        return update_hub_skill(
            config,
            &update_key,
            &record_key,
            &record,
            main_dir,
            download_hub_archive,
        );
    }

    let repo_ref = record.to_repo_ref();
    let repo_root = download_repo_ref(&repo_ref)?;
    let source_dir = resolve_skill_directory(&repo_root, &record.directory);

    if !source_dir.join("SKILL.md").is_file() {
        return Err(AppError::SkillDirNotFound {
            path: source_dir,
        });
    }

    let install_path = resolve_local_library_path(main_dir, &record_key, &record);
    if install_path.exists() {
        ensure_deletable_skill_dir(&install_path, &record_link_name(&record_key, &record))?;
        crate::fs_adapter::delete_real_dir(&install_path)?;
    }

    if let Some(parent) = install_path.parent() {
        fs::create_dir_all(parent).map_err(|err| io_error(Some(parent), err.to_string()))?;
    }

    copy_dir_recursive(&source_dir, &install_path)?;
    let content_hash = compute_dir_hash(&install_path)?;

    if let Some(record) = config.skill_records.get_mut(&record_key) {
        record.content_hash = content_hash;
    }

    config
        .skill_update_cache
        .updates
        .retain(|update| !pending_update_matches(update, &update_key));

    Ok(())
}

fn update_hub_skill<G>(
    config: &mut AppConfig,
    update_key: &str,
    record_key: &str,
    record: &SkillRecord,
    main_dir: &Path,
    download_hub_archive: G,
) -> Result<(), AppError>
where
    G: FnOnce(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    let base_url =
        skill_hub_endpoints::hub_endpoint_base_url(config, &record.hub_endpoint_id)?;
    let zip_path = match download_hub_archive(
        &base_url,
        &record.hub_skill_group,
        &record.hub_skill_id,
    ) {
        Ok(path) => path,
        Err(err) => {
            if matches!(&err, AppError::HubSkillGone { .. }) {
                mark_record_source_missing(config, record_key);
            }
            return Err(err);
        }
    };

    let install_path = resolve_local_library_path(main_dir, record_key, record);
    if install_path.exists() {
        ensure_deletable_skill_dir(&install_path, &record_link_name(record_key, record))?;
        crate::fs_adapter::delete_real_dir(&install_path)?;
    }

    if let Some(parent) = install_path.parent() {
        fs::create_dir_all(parent).map_err(|err| io_error(Some(parent), err.to_string()))?;
    }
    fs::create_dir_all(&install_path).map_err(|err| io_error(Some(&install_path), err.to_string()))?;

    let extract_result = skill_downloader::extract_zip_file(&zip_path, &install_path);
    let _ = fs::remove_file(&zip_path);
    extract_result?;

    if !install_path.join("SKILL.md").is_file() {
        return Err(AppError::SkillDirNotFound {
            path: install_path.join("SKILL.md"),
        });
    }

    let content_hash = compute_skill_md_hash_prefix(&install_path)?;
    if let Some(record) = config.skill_records.get_mut(record_key) {
        record.content_hash = content_hash;
        record.source_missing = false;
    }

    config
        .skill_update_cache
        .updates
        .retain(|update| !pending_update_matches(update, update_key));

    Ok(())
}

pub fn update_all_skills(
    config: &mut AppConfig,
    main_dir: &Path,
) -> Result<UpdateAllSkillsResult, AppError> {
    update_all_skills_with_hooks(
        config,
        main_dir,
        |repo_ref| skill_downloader::download_repo_ref(repo_ref),
        |base_url, group, skill_id| skill_hub_client::download_archive(base_url, group, skill_id),
    )
}

pub fn update_all_skills_with_download_hook<F>(
    config: &mut AppConfig,
    main_dir: &Path,
    mut download_repo_ref: F,
) -> Result<UpdateAllSkillsResult, AppError>
where
    F: FnMut(&RepoRef) -> Result<PathBuf, AppError>,
{
    update_all_skills_with_hooks(config, main_dir, move |repo_ref| download_repo_ref(repo_ref), |_, _, _| {
        Err(AppError::Io {
            path: None,
            message: "Hub 下载未配置".to_string(),
        })
    })
}

pub fn update_all_skills_with_hooks<F, G>(
    config: &mut AppConfig,
    main_dir: &Path,
    mut download_repo_ref: F,
    mut download_hub_archive: G,
) -> Result<UpdateAllSkillsResult, AppError>
where
    F: FnMut(&RepoRef) -> Result<PathBuf, AppError>,
    G: FnMut(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    let pending: Vec<String> = config
        .skill_update_cache
        .updates
        .iter()
        .filter_map(|update| {
            if update.storage_key.is_empty() {
                None
            } else {
                Some(update.storage_key.clone())
            }
        })
        .collect();

    let mut updated = Vec::new();
    let mut failed = Vec::new();

    for dir_name in pending {
        match update_skill_with_hooks(config, &dir_name, main_dir, |repo_ref| {
            download_repo_ref(repo_ref)
        }, |base_url, group, skill_id| download_hub_archive(base_url, group, skill_id)) {
            Ok(()) => updated.push(dir_name),
            Err(AppError::HubSkillGone { .. }) => {
                // Local copy kept; source_missing already marked. Not a hard failure.
            }
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
        let link_name = skill_storage::skill_id_from_directory(directory);
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
            storage_key: link_name.clone(),
            link_name,
            ..Default::default()
        }
    }

    fn config_with_record(record_key: &str, record: SkillRecord) -> AppConfig {
        let mut config = AppConfig::default();
        config.skill_records.insert(record_key.to_string(), record);
        config.skill_repos = vec![SkillRepo {
            host: default_github_host(),
            provider: "github".to_string(),
            project_path: "anthropics/skills".to_string(),
            owner: "anthropics".to_string(),
            name: "skills".to_string(),
            branch: "main".to_string(),
            enabled: true,
        }];
        config
    }

    fn nested_record(storage_key: &str, directory: &str, content_hash: &str) -> SkillRecord {
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
            storage_key: storage_key.to_string(),
            link_name: skill_storage::skill_id_from_directory(directory),
            repo_slug: "github.com--anthropics-skills".to_string(),
            ..Default::default()
        }
    }

    fn nested_install_path(main_dir: &Path, storage_key: &str) -> PathBuf {
        skill_storage::main_library_path(main_dir, storage_key)
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
    fn compute_skill_md_hash_prefix_uses_first_12_hex_chars() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_skill(&temp.path().join("skill"), "example", "a");

        let prefix = compute_skill_md_hash_prefix(&temp.path().join("skill")).expect("hash prefix");
        let full = compute_dir_hash(&temp.path().join("skill")).expect("full hash");

        assert_eq!(prefix.len(), 12);
        assert!(prefix.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(prefix, full);
    }

    #[test]
    fn hub_md_hash_prefix_comparison_matches_server_format() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_skill(&temp.path().join("skill"), "example", "a");
        let skill_dir = temp.path().join("skill");

        let remote = compute_skill_md_hash_prefix(&skill_dir).expect("remote hash");
        let local = local_hash_for_hub_compare(&skill_dir, &remote).expect("local hash");

        assert_eq!(local, remote);
    }

    #[test]
    fn check_updates_skips_records_not_from_configured_skill_repos() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        let repo_root = temp.path().join("repo");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let storage_key = "repo/github.com--obra-superpowers/brainstorming";
        let local_installed =
            crate::skill_storage::main_library_path(&main_dir, storage_key);
        write_valid_skill(&local_installed, "brainstorming", "local-old");
        write_valid_skill(
            &repo_root.join("skills").join("brainstorming"),
            "brainstorming",
            "remote-new",
        );

        let mut config = config_with_record(
            storage_key,
            SkillRecord {
                repo_host: default_github_host(),
                project_path: "obra/superpowers".to_string(),
                source: "github".to_string(),
                repo_owner: "obra".to_string(),
                repo_name: "superpowers".to_string(),
                repo_branch: "main".to_string(),
                directory: "skills/brainstorming".to_string(),
                content_hash: "stale".to_string(),
                installed_at: "2026-06-30T00:00:00Z".to_string(),
                storage_key: storage_key.to_string(),
                link_name: "brainstorming".to_string(),
                repo_slug: "github.com--obra-superpowers".to_string(),
                ..Default::default()
            },
        );
        config.skill_repos = vec![SkillRepo {
            host: "git.xkw.cn".to_string(),
            provider: "gitlab".to_string(),
            project_path: "mp-oxygen/uc/skills".to_string(),
            owner: "mp-oxygen/uc".to_string(),
            name: "skills".to_string(),
            branch: "master".to_string(),
            enabled: true,
        }];

        let download_calls = std::sync::atomic::AtomicUsize::new(0);
        let updates = check_updates_with_download_hook(&mut config, &main_dir, |_| {
            download_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(repo_root.clone())
        })
        .expect("check updates");

        assert!(updates.is_empty());
        assert_eq!(download_calls.load(std::sync::atomic::Ordering::SeqCst), 1);
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

        let updates = check_updates_with_download_hook(&mut config, &main_dir, |_| {
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

        let mut config = config_with_record(
            "brainstorming",
            sample_record("skills/brainstorming", "stale-hash"),
        );

        let updates = check_updates_with_download_hook(&mut config, &main_dir, |_| {
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

        let mut config = config_with_record(
            "brainstorming",
            sample_record("skills/brainstorming", &local_hash),
        );

        let updates =
            check_updates_with_download_hook(&mut config, &main_dir, |_| Ok(repo_root.clone()))
                .expect("check updates");

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].dir_name, "brainstorming");
        assert_eq!(updates[0].storage_key, "brainstorming");
        assert_eq!(updates[0].current_hash.as_deref(), Some(local_hash.as_str()));
        assert_eq!(updates[0].remote_hash, remote_hash);
    }

    #[test]
    fn check_updates_uses_record_map_key_when_storage_key_field_mismatches() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        let repo_root = temp.path().join("repo");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let storage_key = "repo/github.com--anthropics-skills/brainstorming";
        let local_installed = main_dir.join("repo/github.com--anthropics-skills/brainstorming");
        write_valid_skill(&local_installed, "brainstorming", "local-old");
        write_valid_skill(
            &repo_root.join("skills").join("brainstorming"),
            "brainstorming",
            "remote-new",
        );

        let mut record = sample_record("skills/brainstorming", "stale-hash");
        // Stale field: bare link name while map key is the full storage key.
        record.storage_key = "brainstorming".to_string();
        let mut config = config_with_record(storage_key, record);

        let updates =
            check_updates_with_download_hook(&mut config, &main_dir, |_| Ok(repo_root.clone()))
                .expect("check updates");

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].storage_key, storage_key);
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
                storage_key: "brainstorming".to_string(),
                ..Default::default()
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
                    storage_key: "good".to_string(),
                    ..Default::default()
                },
                SkillUpdateInfo {
                    dir_name: "bad".to_string(),
                    name: "bad".to_string(),
                    current_hash: Some("stale".to_string()),
                    remote_hash: "deadbeef".to_string(),
                    storage_key: "bad".to_string(),
                    ..Default::default()
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

    #[test]
    fn check_updates_marks_hub_skill_gone_as_source_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let storage_key = "hub/company-hub/tools/brainstorming";
        let local_installed = nested_install_path(&main_dir, storage_key);
        write_valid_skill(&local_installed, "brainstorming", "local");

        let mut config = config_with_record(
            storage_key,
            SkillRecord {
                source: "skillhub".to_string(),
                directory: "tools/brainstorming".to_string(),
                content_hash: "local".to_string(),
                installed_at: "2026-06-30T00:00:00Z".to_string(),
                storage_key: storage_key.to_string(),
                link_name: "brainstorming".to_string(),
                hub_endpoint_id: "company-hub".to_string(),
                hub_skill_group: "tools".to_string(),
                hub_skill_id: "brainstorming".to_string(),
                ..Default::default()
            },
        );
        config.skill_hub_endpoints = vec![crate::models::SkillHubEndpoint {
            id: "company-hub".to_string(),
            name: "Company Hub".to_string(),
            base_url: "https://hub.example.com".to_string(),
            enabled: true,
        }];
        config.skill_update_cache = SkillUpdateCache {
            checked_at: Some("2026-06-30T00:00:00Z".to_string()),
            updates: vec![SkillUpdateInfo {
                dir_name: "brainstorming".to_string(),
                name: "brainstorming".to_string(),
                current_hash: Some("local".to_string()),
                remote_hash: "remote".to_string(),
                storage_key: storage_key.to_string(),
                ..Default::default()
            }],
        };

        let updates = check_updates_with_hooks(
            &mut config,
            &main_dir,
            |_| {
                Err(AppError::Io {
                    path: None,
                    message: "unused".to_string(),
                })
            },
            |_, _, _| {
                Err(AppError::HubSkillGone {
                    skill_id: "brainstorming".to_string(),
                    group: "tools".to_string(),
                })
            },
        )
        .expect("check updates should not fail on hub gone");

        assert!(updates.is_empty());
        assert!(
            config
                .skill_records
                .get(storage_key)
                .expect("record")
                .source_missing
        );
        assert!(config.skill_update_cache.updates.is_empty());
    }

    #[test]
    fn check_updates_detects_hash_change_for_nested_storage_key() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        let repo_root = temp.path().join("repo");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let storage_key = "repo/github.com--anthropics-skills/brainstorming";
        let local_installed = nested_install_path(&main_dir, storage_key);
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

        let mut config = config_with_record(
            storage_key,
            nested_record(storage_key, "skills/brainstorming", &local_hash),
        );

        let updates =
            check_updates_with_download_hook(&mut config, &main_dir, |_| Ok(repo_root.clone()))
                .expect("check updates");

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].dir_name, "brainstorming");
        assert_eq!(updates[0].storage_key, storage_key);
        assert_eq!(updates[0].current_hash.as_deref(), Some(local_hash.as_str()));
        assert_eq!(updates[0].remote_hash, remote_hash);
    }

    #[test]
    fn update_skill_overwrites_nested_storage_key_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        let repo_root = temp.path().join("repo");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let storage_key = "repo/github.com--anthropics-skills/brainstorming";
        let install_path = nested_install_path(&main_dir, storage_key);
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
            storage_key,
            nested_record(storage_key, "skills/brainstorming", "stale-hash"),
        );
        config.skill_update_cache = SkillUpdateCache {
            checked_at: Some("2026-06-30T00:00:00Z".to_string()),
            updates: vec![SkillUpdateInfo {
                dir_name: "brainstorming".to_string(),
                name: "brainstorming".to_string(),
                current_hash: Some("stale-hash".to_string()),
                remote_hash: remote_hash.clone(),
                storage_key: storage_key.to_string(),
                ..Default::default()
            }],
        };

        update_skill_with_download_hook(&mut config, storage_key, &main_dir, |_| {
            Ok(repo_root.clone())
        })
        .expect("update skill");

        assert!(!install_path.join("stale.txt").exists());
        assert_eq!(fs::read_to_string(install_path.join("fresh.txt")).unwrap(), "new");
        assert_eq!(
            config
                .skill_records
                .get(storage_key)
                .expect("record")
                .content_hash,
            remote_hash
        );
        assert!(config.skill_update_cache.updates.is_empty());
    }
}
    #[test]
    fn strict_repo_update_check_calls_only_selected_provider() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("main dir");
        let mut config = AppConfig::default();
        config.skill_repos = vec![
            SkillRepo {
                host: "github.com".to_string(),
                provider: "github".to_string(),
                project_path: "owner/github".to_string(),
                owner: "owner".to_string(),
                name: "github".to_string(),
                branch: "main".to_string(),
                enabled: true,
            },
            SkillRepo {
                host: "gitlab.internal".to_string(),
                provider: "gitlab".to_string(),
                project_path: "team/gitlab".to_string(),
                owner: "team".to_string(),
                name: "gitlab".to_string(),
                branch: "main".to_string(),
                enabled: true,
            },
        ];
        let mut calls = Vec::new();

        let updates = check_repo_updates_strict_with_hook(
            &config,
            &main_dir,
            "gitlab",
            |repo| {
                calls.push(repo.provider.clone());
                Ok(temp.path().to_path_buf())
            },
        )
        .expect("strict repo updates");

        assert!(updates.is_empty());
        assert_eq!(calls, vec!["gitlab"]);
    }

    #[test]
    fn strict_hub_update_check_propagates_enabled_endpoint_failure() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("main dir");
        let mut config = AppConfig::default();
        config.skill_hub_endpoints.push(crate::models::SkillHubEndpoint {
            id: "company-hub".to_string(),
            name: "Company Hub".to_string(),
            base_url: "https://hub.internal".to_string(),
            enabled: true,
        });
        config.skill_records.insert(
            "hub-key".to_string(),
            SkillRecord {
                source: "skillhub".to_string(),
                storage_key: "hub-key".to_string(),
                hub_endpoint_id: "company-hub".to_string(),
                hub_skill_group: "tools".to_string(),
                hub_skill_id: "tdd".to_string(),
                ..SkillRecord::default()
            },
        );

        let result = check_hub_updates_strict_with_hook(
            &mut config,
            &main_dir,
            |_, _, _| {
                Err(AppError::Io {
                    path: None,
                    message: "hub unavailable".to_string(),
                })
            },
        );

        assert!(result.is_err());
    }
