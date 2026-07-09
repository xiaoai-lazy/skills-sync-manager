use crate::link_installer;
use crate::models::{AppConfig, AppError, Installation, SkillRecord, default_github_host};
use crate::skill_discover::iso8601_timestamp_now;
use crate::skill_downloader::copy_dir_recursive;
use crate::skill_storage;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const CONTAINER_DIRS: &[&str] = &["repo", "hub", "local"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V5MigrationPlanItem {
    pub old_key: String,
    pub storage_key: String,
    pub link_name: String,
    pub repo_slug: String,
    pub old_path: PathBuf,
    pub new_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V5MoveResult {
    pub succeeded: Vec<V5MigrationPlanItem>,
    pub failed: Vec<V5MoveFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V5MoveFailure {
    pub item: V5MigrationPlanItem,
    pub reason: String,
}

pub fn plan_v5_record(old_key: &str, record: &SkillRecord, main_dir: &Path) -> V5MigrationPlanItem {
    let skill_id = if record.directory.is_empty() {
        old_key.to_string()
    } else {
        skill_storage::skill_id_from_directory(&record.directory)
    };
    let link_name = skill_id.clone();

    let repo_host = if record.repo_host.is_empty() {
        default_github_host()
    } else {
        record.repo_host.clone()
    };

    let project_path = if record.project_path.is_empty() {
        format!("{}/{}", record.repo_owner, record.repo_name)
    } else {
        record.project_path.clone()
    };

    let repo_slug = if !record.repo_slug.is_empty() {
        record.repo_slug.clone()
    } else if matches!(record.source.as_str(), "github" | "gitlab" | "skillssh") {
        skill_storage::compute_repo_slug(&repo_host, &project_path)
    } else {
        String::new()
    };

    let storage_key = skill_storage::storage_key_from_record_source(
        &record.source,
        if repo_slug.is_empty() {
            None
        } else {
            Some(&repo_slug)
        },
        if record.hub_endpoint_id.is_empty() {
            None
        } else {
            Some(&record.hub_endpoint_id)
        },
        if record.hub_skill_group.is_empty() {
            None
        } else {
            Some(&record.hub_skill_group)
        },
        &skill_id,
    );
    let new_path = skill_storage::main_library_path(main_dir, &storage_key);

    V5MigrationPlanItem {
        old_key: old_key.to_string(),
        storage_key,
        link_name,
        repo_slug,
        old_path: main_dir.join(old_key),
        new_path,
    }
}

pub fn plan_v5_migration(config: &AppConfig, main_dir: &Path) -> Vec<V5MigrationPlanItem> {
    config
        .skill_records
        .iter()
        .map(|(old_key, record)| plan_v5_record(old_key, record, main_dir))
        .collect()
}

pub fn plan_orphan_local_dirs(
    main_dir: &Path,
    planned_old_paths: &HashSet<PathBuf>,
) -> Vec<V5MigrationPlanItem> {
    let entries = match fs::read_dir(main_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut items = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|part| part.to_str()) else {
            continue;
        };

        if name.starts_with('.') || CONTAINER_DIRS.contains(&name) {
            continue;
        }

        if planned_old_paths.contains(&path) {
            continue;
        }

        let storage_key = skill_storage::storage_key_for_local(name);
        items.push(V5MigrationPlanItem {
            old_key: name.to_string(),
            storage_key: storage_key.clone(),
            link_name: name.to_string(),
            repo_slug: String::new(),
            old_path: path,
            new_path: skill_storage::main_library_path(main_dir, &storage_key),
        });
    }

    items
}

pub fn backup_main_library(main_dir: &Path, timestamp: &str) -> Result<PathBuf, AppError> {
    let parent = main_dir.parent().ok_or_else(|| AppError::Io {
        path: Some(main_dir.to_path_buf()),
        message: "主技能库路径没有父目录".to_string(),
    })?;
    let dir_name = main_dir.file_name().ok_or_else(|| AppError::Io {
        path: Some(main_dir.to_path_buf()),
        message: "主技能库路径没有目录名".to_string(),
    })?;

    let backup_path = parent.join(format!(
        "{}.backup-v5-{}",
        dir_name.to_string_lossy(),
        timestamp
    ));

    if backup_path.exists() {
        // Resume after an interrupted migration attempt that already created the backup.
        return Ok(backup_path);
    }

    copy_dir_recursive(main_dir, &backup_path)?;
    Ok(backup_path)
}

fn is_cross_device_error(err: &std::io::Error) -> bool {
    matches!(
        err.raw_os_error(),
        Some(18) | Some(17) // EXDEV (unix) / ERROR_NOT_SAME_DEVICE (windows)
    ) || err
        .to_string()
        .to_ascii_lowercase()
        .contains("cross-device")
}

fn verify_move_destination(item: &V5MigrationPlanItem) -> Result<(), String> {
    if !item.new_path.is_dir() {
        return Err(format!(
            "迁移目标不是目录: {}",
            item.new_path.display()
        ));
    }

    Ok(())
}

fn move_item(item: &V5MigrationPlanItem) -> Result<(), String> {
    if !item.old_path.exists() {
        if item.new_path.exists() {
            return Ok(());
        }
        return Err(format!(
            "源目录不存在: {}",
            item.old_path.display()
        ));
    }

    if item.new_path.exists() {
        return Err(format!(
            "目标目录已存在: {}",
            item.new_path.display()
        ));
    }

    if let Some(parent) = item.new_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "创建目标父目录失败 {}: {}",
                parent.display(),
                err
            )
        })?;
    }

    match fs::rename(&item.old_path, &item.new_path) {
        Ok(()) => Ok(()),
        Err(err) if is_cross_device_error(&err) => {
            copy_dir_recursive(&item.old_path, &item.new_path).map_err(|copy_err| {
                format!(
                    "跨设备复制失败 {} -> {}: {}",
                    item.old_path.display(),
                    item.new_path.display(),
                    copy_err
                )
            })?;
            verify_move_destination(item)?;
            fs::remove_dir_all(&item.old_path).map_err(|remove_err| {
                format!(
                    "复制后删除源目录失败 {}: {}",
                    item.old_path.display(),
                    remove_err
                )
            })?;
            Ok(())
        }
        Err(err) => Err(format!(
            "重命名失败 {} -> {}: {}",
            item.old_path.display(),
            item.new_path.display(),
            err
        )),
    }
}

pub fn execute_moves(items: &[V5MigrationPlanItem]) -> V5MoveResult {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    for item in items {
        match move_item(item) {
            Ok(()) => succeeded.push(item.clone()),
            Err(reason) => failed.push(V5MoveFailure {
                item: item.clone(),
                reason,
            }),
        }
    }

    V5MoveResult { succeeded, failed }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    pub backed_up_config: PathBuf,
    pub backed_up_main: Option<PathBuf>,
    pub succeeded: Vec<String>,
    pub failed: Vec<String>,
    pub orphan_locals: Vec<String>,
    pub links_repaired: u32,
}

impl MigrationReport {
    fn empty() -> Self {
        Self {
            backed_up_config: PathBuf::new(),
            backed_up_main: None,
            succeeded: Vec::new(),
            failed: Vec::new(),
            orphan_locals: Vec::new(),
            links_repaired: 0,
        }
    }
}

impl From<MigrationReport> for crate::models::MigrationReportDto {
    fn from(report: MigrationReport) -> Self {
        Self {
            backed_up_config: report.backed_up_config.display().to_string(),
            backed_up_main: report
                .backed_up_main
                .map(|path| path.display().to_string()),
            succeeded: report.succeeded,
            failed: report.failed,
            orphan_locals: report.orphan_locals,
            links_repaired: report.links_repaired,
        }
    }
}

pub fn backup_config_before_migration(config_path: &Path) -> Result<PathBuf, AppError> {
    let backup_path = config_path.with_extension("json.backup-v5");
    fs::copy(config_path, &backup_path).map_err(|err| AppError::Io {
        path: Some(backup_path.clone()),
        message: format!("failed to backup config before v5 migration: {}", err),
    })?;
    Ok(backup_path)
}

fn migration_timestamp() -> String {
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn main_dir_has_entries(main_dir: &Path) -> bool {
    fs::read_dir(main_dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

pub fn migrate_v5_to_v6(
    config: &mut AppConfig,
    main_dir: &Path,
    config_path: &Path,
) -> Result<MigrationReport, AppError> {
    if config.version >= 6 {
        return Ok(MigrationReport::empty());
    }

    let backed_up_config = backup_config_before_migration(config_path)?;

    let backed_up_main = if main_dir.is_dir() && main_dir_has_entries(main_dir) {
        Some(backup_main_library(main_dir, &migration_timestamp())?)
    } else {
        None
    };

    // Do not detach existing target links during migration: deleting junctions can hit
    // Windows access-denied when Cursor/agents hold the path. After the main-library
    // move, stale links are left for the user to remove manually; missing links are
    // recreated below, and again on later startups.
    let planned = plan_v5_migration(config, main_dir);
    let planned_old_paths: HashSet<PathBuf> = planned.iter().map(|item| item.old_path.clone()).collect();
    let orphan_plans = plan_orphan_local_dirs(main_dir, &planned_old_paths);
    let orphan_locals: Vec<String> = orphan_plans
        .iter()
        .map(|item| item.old_key.clone())
        .collect();

    let mut all_items = planned;
    all_items.extend(orphan_plans);

    let move_result = execute_moves(&all_items);
    if !move_result.failed.is_empty() {
        let failed: Vec<String> = move_result
            .failed
            .iter()
            .map(|failure| format!("{}: {}", failure.item.old_key, failure.reason))
            .collect();
        return Err(AppError::Io {
            path: None,
            message: format!("v5→v6 迁移移动失败: {}", failed.join("; ")),
        });
    }

    apply_config_after_moves(config, &move_result.succeeded, &orphan_locals);
    let (links_repaired, link_failures) = recreate_all_installation_links(config);

    config.version = 6;

    let succeeded: Vec<String> = move_result
        .succeeded
        .iter()
        .map(|item| item.storage_key.clone())
        .collect();

    let report = MigrationReport {
        backed_up_config,
        backed_up_main,
        succeeded,
        failed: link_failures,
        orphan_locals,
        links_repaired,
    };

    write_migration_log(config_path, &report)?;
    Ok(report)
}

fn apply_config_after_moves(
    config: &mut AppConfig,
    succeeded: &[V5MigrationPlanItem],
    orphan_locals: &[String],
) {
    let succeeded_by_old_key: HashMap<&str, &V5MigrationPlanItem> = succeeded
        .iter()
        .map(|item| (item.old_key.as_str(), item))
        .collect();
    let orphan_set: HashSet<&str> = orphan_locals.iter().map(String::as_str).collect();

    let mut new_records = HashMap::new();
    for (old_key, mut record) in config.skill_records.drain() {
        if let Some(item) = succeeded_by_old_key.get(old_key.as_str()) {
            record.storage_key = item.storage_key.clone();
            record.link_name = item.link_name.clone();
            if record.repo_slug.is_empty() && !item.repo_slug.is_empty() {
                record.repo_slug = item.repo_slug.clone();
            }
            new_records.insert(item.storage_key.clone(), record);
        } else {
            new_records.insert(old_key, record);
        }
    }

    for item in succeeded {
        if !orphan_set.contains(item.old_key.as_str()) {
            continue;
        }
        if new_records.contains_key(&item.storage_key) {
            continue;
        }
        new_records.insert(
            item.storage_key.clone(),
            SkillRecord {
                source: "local".to_string(),
                directory: item.link_name.clone(),
                storage_key: item.storage_key.clone(),
                link_name: item.link_name.clone(),
                installed_at: iso8601_timestamp_now(),
                ..Default::default()
            },
        );
    }

    config.skill_records = new_records;

    for installation in &mut config.installations {
        update_installation_after_move(installation, succeeded);
    }
}

fn update_installation_after_move(
    installation: &mut Installation,
    succeeded: &[V5MigrationPlanItem],
) {
    for item in succeeded {
        if link_installer::same_path(&installation.source_path, &item.old_path)
            || installation.skill_dir_name == item.old_key
            || installation.skill_storage_key == item.old_key
        {
            installation.source_path = item.new_path.clone();
            installation.skill_storage_key = item.storage_key.clone();
            if installation.skill_dir_name == item.old_key {
                installation.skill_dir_name = item.link_name.clone();
            }
            return;
        }
    }
}

fn recreate_all_installation_links(config: &AppConfig) -> (u32, Vec<String>) {
    let mut repaired = 0;
    let mut failures = Vec::new();
    for installation in &config.installations {
        match repair_installation_link(installation) {
            Ok(true) => repaired += 1,
            Ok(false) => {}
            Err(err) => {
                let message = format!(
                    "{}: 重建链接失败: {}",
                    installation.link_path.display(),
                    err
                );
                eprintln!("[migration] {}", message);
                failures.push(message);
            }
        }
    }
    (repaired, failures)
}

/// Repair installation links that are missing (source still exists).
/// Does not delete or rewrite existing wrong links — those require manual removal.
pub fn repair_stale_installation_links(config: &AppConfig) -> (u32, Vec<String>) {
    recreate_all_installation_links(config)
}

/// Returns true when at least one installation has a missing link that can be recreated
/// (source exists, link path does not). Wrong existing links are not considered repairable.
pub fn has_stale_installation_links(config: &AppConfig) -> bool {
    for installation in &config.installations {
        if !crate::fs_adapter::path_exists(&installation.source_path) {
            continue;
        }
        if !crate::fs_adapter::path_exists(&installation.link_path) {
            return true;
        }
    }
    false
}

/// Drop installation records whose `source_path` no longer exists in the main library.
/// Returns how many records were removed.
pub fn purge_installations_with_missing_source(config: &mut AppConfig) -> u32 {
    let before = config.installations.len();
    config
        .installations
        .retain(|installation| crate::fs_adapter::path_exists(&installation.source_path));
    (before - config.installations.len()) as u32
}

fn repair_installation_link(installation: &Installation) -> Result<bool, AppError> {
    link_installer::repair_installation_link(installation, &installation.source_path)
}

fn write_migration_log(config_path: &Path, report: &MigrationReport) -> Result<(), AppError> {
    let log_path = config_path
        .parent()
        .map(|parent| parent.join("migration-v5-v6.log"))
        .unwrap_or_else(|| PathBuf::from("migration-v5-v6.log"));

    let content = format!(
        "v5→v6 migration\n\
         backed up config: {}\n\
         backed up main: {}\n\
         succeeded: {}\n\
         failed: {}\n\
         orphan locals: {}\n\
         links repaired: {}\n",
        report.backed_up_config.display(),
        report
            .backed_up_main
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "(none)".to_string()),
        report.succeeded.join(", "),
        report.failed.join("; "),
        report.orphan_locals.join(", "),
        report.links_repaired,
    );

    fs::write(&log_path, content).map_err(|err| AppError::Io {
        path: Some(log_path),
        message: format!("failed to write migration log: {}", err),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AppConfig, AppError, Installation, Target,
    };
    use std::collections::HashSet;

    fn github_tdd_record() -> SkillRecord {
        SkillRecord {
            source: "github".to_string(),
            repo_host: "github.com".to_string(),
            project_path: "anthropics/skills".to_string(),
            repo_owner: "anthropics".to_string(),
            repo_name: "skills".to_string(),
            repo_branch: "main".to_string(),
            directory: "skills/tdd".to_string(),
            content_hash: "abc".to_string(),
            installed_at: "2026-01-01".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn migration_plan_maps_flat_key_to_storage_key() {
        let record = github_tdd_record();
        let main = PathBuf::from("/main");
        let plan = plan_v5_record("tdd", &record, &main);
        assert_eq!(
            plan.storage_key,
            "repo/github.com--anthropics-skills/tdd"
        );
        assert_eq!(plan.link_name, "tdd");
        assert_eq!(plan.old_path, PathBuf::from("/main/tdd"));
        assert_eq!(
            plan.new_path,
            PathBuf::from("/main/repo/github.com--anthropics-skills/tdd")
        );
    }

    #[test]
    fn plan_v5_migration_iterates_skill_records() {
        let mut config = AppConfig::default();
        config
            .skill_records
            .insert("tdd".to_string(), github_tdd_record());

        let main = PathBuf::from("/main");
        let plans = plan_v5_migration(&config, &main);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].old_key, "tdd");
    }

    #[test]
    fn plan_orphan_local_dirs_maps_unknown_top_level_dirs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        fs::create_dir_all(&main).expect("create main");
        fs::create_dir_all(main.join("orphan")).expect("create orphan");
        fs::create_dir_all(main.join("repo")).expect("create repo container");

        let planned = HashSet::from([main.join("tdd")]);
        let plans = plan_orphan_local_dirs(&main, &planned);

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].old_key, "orphan");
        assert_eq!(plans[0].storage_key, "local/orphan");
    }

    #[test]
    fn backup_main_library_copies_entire_main_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main-skills");
        fs::create_dir_all(&main).expect("create main");
        fs::write(main.join("SKILL.md"), "backup me").expect("write skill");

        let backup =
            backup_main_library(&main, "20260101-120000").expect("backup main library");

        assert!(backup.exists());
        assert_eq!(
            fs::read_to_string(backup.join("SKILL.md")).expect("read backup"),
            "backup me"
        );
    }

    #[test]
    fn backup_main_library_reuses_existing_backup() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main-skills");
        fs::create_dir_all(&main).expect("create main");
        fs::write(main.join("SKILL.md"), "backup me").expect("write skill");

        let first =
            backup_main_library(&main, "20260101-120000").expect("first backup");
        fs::write(main.join("SKILL.md"), "changed after backup").expect("rewrite skill");

        let second =
            backup_main_library(&main, "20260101-120000").expect("reuse backup");
        assert_eq!(first, second);
        assert_eq!(
            fs::read_to_string(second.join("SKILL.md")).expect("read backup"),
            "backup me"
        );
    }

    #[test]
    fn execute_moves_renames_flat_to_nested() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        let old_path = main.join("tdd");
        fs::create_dir_all(&old_path).expect("create old path");
        fs::write(old_path.join("SKILL.md"), "# TDD").expect("write skill");

        let plan = plan_v5_record("tdd", &github_tdd_record(), &main);
        let result = execute_moves(&[plan]);

        assert_eq!(result.succeeded.len(), 1);
        assert!(result.failed.is_empty());
        assert!(!old_path.exists());
        assert!(main.join("repo/github.com--anthropics-skills/tdd/SKILL.md").exists());
    }

    #[test]
    fn execute_moves_fails_when_target_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        let old_path = main.join("tdd");
        let new_path = main.join("repo/github.com--anthropics-skills/tdd");

        fs::create_dir_all(&old_path).expect("create old path");
        fs::write(old_path.join("SKILL.md"), "old").expect("write old skill");
        fs::create_dir_all(&new_path).expect("create new path");
        fs::write(new_path.join("SKILL.md"), "new").expect("write new skill");

        let plan = plan_v5_record("tdd", &github_tdd_record(), &main);
        let result = execute_moves(&[plan]);

        assert!(result.succeeded.is_empty());
        assert_eq!(result.failed.len(), 1);
        assert!(old_path.exists());
        assert!(new_path.exists());
        assert_eq!(
            fs::read_to_string(old_path.join("SKILL.md")).expect("read old"),
            "old"
        );
    }

    #[test]
    fn execute_moves_is_idempotent_when_already_moved() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        let new_path = main.join("repo/github.com--anthropics-skills/tdd");

        fs::create_dir_all(&new_path).expect("create new path");
        fs::write(new_path.join("SKILL.md"), "already moved").expect("write skill");

        let plan = plan_v5_record("tdd", &github_tdd_record(), &main);
        let result = execute_moves(&[plan]);

        assert_eq!(result.succeeded.len(), 1);
        assert!(result.failed.is_empty());
        assert!(new_path.join("SKILL.md").exists());
    }

    #[test]
    fn migrate_v5_flat_layout_to_v6() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        let flat = main.join("tdd");
        fs::create_dir_all(&flat).expect("create flat");
        fs::write(
            flat.join("SKILL.md"),
            "---\nname: tdd\ndescription: Test.\n---\n\n# TDD\n",
        )
        .expect("write skill");

        let config_path = temp.path().join("config.json");
        fs::write(&config_path, r#"{"version":5}"#).expect("write config");

        let mut config = AppConfig::default();
        config.version = 5;
        config.settings.main_skills_dir = Some(main.clone());
        config
            .skill_records
            .insert("tdd".to_string(), github_tdd_record());

        let target_dir = temp.path().join("target");
        fs::create_dir_all(&target_dir).expect("create target");
        let link_path = target_dir.join("tdd");
        crate::fs_adapter::create_dir_link(
            &flat,
            &link_path,
            crate::fs_adapter::default_link_type(),
        )
        .expect("create pre-migration link");

        config.targets.push(Target::global_custom(
            "t1",
            "Target",
            target_dir.clone(),
            "1",
            "1",
        ));
        config.installations.push(Installation {
            id: "i1".to_string(),
            skill_dir_name: "tdd".to_string(),
            skill_name: "tdd".to_string(),
            source_path: flat.clone(),
            target_id: "t1".to_string(),
            link_path: link_path.clone(),
            link_type: crate::fs_adapter::default_link_type(),
            created_at: "1".to_string(),
            ..Default::default()
        });

        let report = migrate_v5_to_v6(&mut config, &main, &config_path).expect("migrate");

        let nested = main.join("repo/github.com--anthropics-skills/tdd");
        assert_eq!(config.version, 6);
        assert!(!flat.exists());
        assert!(nested.join("SKILL.md").exists());
        assert!(config.skill_records.contains_key("repo/github.com--anthropics-skills/tdd"));
        assert!(!config.skill_records.contains_key("tdd"));
        assert_eq!(config.installations[0].source_path, nested);
        assert_eq!(
            config.installations[0].skill_storage_key,
            "repo/github.com--anthropics-skills/tdd"
        );
        assert_eq!(config.installations[0].link_path, link_path);
        assert!(config_path.with_extension("json.backup-v5").exists());
        assert_eq!(report.succeeded, vec!["repo/github.com--anthropics-skills/tdd"]);
        assert!(report.failed.is_empty());
        // Existing pre-migration link is left alone (may still point at old flat path).
        // Only missing links are recreated; user deletes stale links manually.
        assert_eq!(report.links_repaired, 0);
        assert!(crate::fs_adapter::path_exists(&link_path));
    }

    #[test]
    fn repair_stale_links_recreates_missing_link_only() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        let nested = main.join("repo/github.com--anthropics-skills/tdd");
        fs::create_dir_all(&nested).expect("create nested");
        fs::write(nested.join("SKILL.md"), "ok").expect("write skill");

        let target_dir = temp.path().join("target");
        fs::create_dir_all(&target_dir).expect("create target");
        let link_path = target_dir.join("tdd");

        let mut config = AppConfig::default();
        config.version = 6;
        config.installations.push(Installation {
            id: "i1".to_string(),
            skill_dir_name: "tdd".to_string(),
            skill_name: "tdd".to_string(),
            source_path: nested.clone(),
            target_id: "t1".to_string(),
            link_path: link_path.clone(),
            link_type: crate::fs_adapter::default_link_type(),
            created_at: "1".to_string(),
            skill_storage_key: "repo/github.com--anthropics-skills/tdd".to_string(),
            ..Default::default()
        });

        assert!(has_stale_installation_links(&config));
        let (repaired, failures) = repair_stale_installation_links(&config);
        assert!(failures.is_empty());
        assert_eq!(repaired, 1);
        assert!(!has_stale_installation_links(&config));
        let actual = crate::fs_adapter::link_target(&link_path)
            .expect("read link")
            .expect("link exists");
        assert!(link_installer::same_path(&actual, &nested));
    }

    #[test]
    fn repair_stale_links_leaves_mismatch_link_untouched() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        let nested = main.join("repo/github.com--anthropics-skills/tdd");
        fs::create_dir_all(&nested).expect("create nested");
        fs::write(nested.join("SKILL.md"), "ok").expect("write skill");

        let old_flat = main.join("tdd");
        fs::create_dir_all(&old_flat).expect("create old flat");
        fs::write(old_flat.join("SKILL.md"), "old").expect("write old");

        let target_dir = temp.path().join("target");
        fs::create_dir_all(&target_dir).expect("create target");
        let link_path = target_dir.join("tdd");
        crate::fs_adapter::create_dir_link(
            &old_flat,
            &link_path,
            crate::fs_adapter::default_link_type(),
        )
        .expect("create stale link");

        let mut config = AppConfig::default();
        config.version = 6;
        config.installations.push(Installation {
            id: "i1".to_string(),
            skill_dir_name: "tdd".to_string(),
            skill_name: "tdd".to_string(),
            source_path: nested.clone(),
            target_id: "t1".to_string(),
            link_path: link_path.clone(),
            link_type: crate::fs_adapter::default_link_type(),
            created_at: "1".to_string(),
            skill_storage_key: "repo/github.com--anthropics-skills/tdd".to_string(),
            ..Default::default()
        });

        assert!(!has_stale_installation_links(&config));
        let (repaired, failures) = repair_stale_installation_links(&config);
        assert!(failures.is_empty());
        assert_eq!(repaired, 0);
        let actual = crate::fs_adapter::link_target(&link_path)
            .expect("read link")
            .expect("link exists");
        assert!(link_installer::same_path(&actual, &old_flat));
    }

    #[test]
    fn purge_installations_with_missing_source_removes_orphan_records() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        let existing = main.join("keep");
        fs::create_dir_all(&existing).expect("create keep");

        let mut config = AppConfig::default();
        config.installations.push(Installation {
            id: "gone".to_string(),
            skill_dir_name: "gone".to_string(),
            skill_name: "gone".to_string(),
            source_path: main.join("missing-skill"),
            target_id: "t1".to_string(),
            link_path: temp.path().join("target/gone"),
            link_type: crate::fs_adapter::default_link_type(),
            created_at: "1".to_string(),
            ..Default::default()
        });
        config.installations.push(Installation {
            id: "keep".to_string(),
            skill_dir_name: "keep".to_string(),
            skill_name: "keep".to_string(),
            source_path: existing.clone(),
            target_id: "t1".to_string(),
            link_path: temp.path().join("target/keep"),
            link_type: crate::fs_adapter::default_link_type(),
            created_at: "1".to_string(),
            ..Default::default()
        });

        let purged = purge_installations_with_missing_source(&mut config);
        assert_eq!(purged, 1);
        assert_eq!(config.installations.len(), 1);
        assert_eq!(config.installations[0].id, "keep");
    }

    #[test]
    fn migrate_v5_aborts_on_move_failure() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        let flat = main.join("tdd");
        let conflict = main.join("repo/github.com--anthropics-skills/tdd");

        fs::create_dir_all(&flat).expect("create flat");
        fs::write(flat.join("SKILL.md"), "old").expect("write old skill");
        fs::create_dir_all(&conflict).expect("create conflict");
        fs::write(conflict.join("SKILL.md"), "new").expect("write conflict skill");

        let config_path = temp.path().join("config.json");
        fs::write(&config_path, r#"{"version":5}"#).expect("write config");

        let mut config = AppConfig::default();
        config.version = 5;
        config.settings.main_skills_dir = Some(main.clone());
        config
            .skill_records
            .insert("tdd".to_string(), github_tdd_record());

        let error = migrate_v5_to_v6(&mut config, &main, &config_path).expect_err("should fail");

        assert!(matches!(error, AppError::Io { .. }));
        assert_eq!(config.version, 5);
        assert!(flat.exists());
        assert!(conflict.exists());
        assert!(config.skill_records.contains_key("tdd"));
    }
}
