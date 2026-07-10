use crate::models::{AppConfig, AppError, DeleteMainSkillResult, Installation, SkillRecord};
use crate::skill_storage;

pub fn delete_main_skill(
    config: &mut AppConfig,
    identifier: &str,
    confirmed: bool,
) -> Result<DeleteMainSkillResult, AppError> {
    if !confirmed {
        return Err(AppError::Io {
            path: None,
            message: "删除主目录 skill 需要确认".to_string(),
        });
    }

    validate_skill_identifier(identifier)?;

    let main_dir = config
        .settings
        .main_skills_dir
        .as_ref()
        .ok_or_else(|| AppError::Io {
            path: None,
            message: "未配置主 skill 目录".to_string(),
        })?;

    let (source_path, record_key, link_name) =
        resolve_delete_target(config, identifier, main_dir)?;

    // Check for symlinks before is_dir to avoid following junctions/symlinks
    match std::fs::symlink_metadata(&source_path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(AppError::Io {
                path: Some(source_path.clone()),
                message: format!("源 skill 路径是链接，无法删除：{}", link_name),
            });
        }
        _ => {}
    }

    if !source_path.is_dir() {
        return Err(AppError::Io {
            path: Some(source_path.clone()),
            message: format!("源 skill 目录不存在：{}", link_name),
        });
    }

    let related: Vec<Installation> = config
        .installations
        .iter()
        .filter(|installation| {
            installation_matches_delete(installation, &source_path, &record_key, &link_name, identifier)
        })
        .cloned()
        .collect();

    for installation in &related {
        if !crate::fs_adapter::path_exists(&installation.link_path) {
            continue;
        }
        match crate::fs_adapter::remove_recorded_link(
            &installation.link_path,
            &installation.source_path,
        ) {
            Ok(()) => {}
            // Leave drifted links for the user to delete manually.
            Err(err) if err.to_string().contains("指向的目标与记录不符") => {}
            Err(err) => return Err(err),
        }
    }

    crate::fs_adapter::delete_real_dir(&source_path)?;
    prune_empty_storage_parents(main_dir, &source_path);

    config.installations.retain(|installation| {
        !installation_matches_delete(installation, &source_path, &record_key, &link_name, identifier)
    });

    cleanup_skill_hub_metadata(config, &record_key);

    Ok(DeleteMainSkillResult {
        deleted_skill_dir_name: link_name,
        removed_link_count: related.len(),
    })
}

/// Remove empty ancestor directories under the main library after deleting a nested skill
/// (e.g. `repo/<slug>/` and `repo/` when the last leaf is gone). Stops at `main_dir`.
fn prune_empty_storage_parents(main_dir: &std::path::Path, deleted_path: &std::path::Path) {
    let mut current = deleted_path.parent().map(|p| p.to_path_buf());
    while let Some(dir) = current {
        if dir == main_dir || !dir.starts_with(main_dir) {
            break;
        }
        let is_empty = std::fs::read_dir(&dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if !is_empty {
            break;
        }
        if std::fs::remove_dir(&dir).is_err() {
            break;
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
}

fn resolve_local_library_path(
    main_dir: &std::path::Path,
    record_key: &str,
    record: &SkillRecord,
) -> std::path::PathBuf {
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

fn resolve_delete_target(
    config: &AppConfig,
    identifier: &str,
    main_dir: &std::path::Path,
) -> Result<(std::path::PathBuf, String, String), AppError> {
    if let Some(record) = config.skill_records.get(identifier) {
        let link_name = record_link_name(identifier, record);
        return Ok((
            resolve_local_library_path(main_dir, identifier, record),
            identifier.to_string(),
            link_name,
        ));
    }

    if let Some((record_key, record)) = config
        .skill_records
        .iter()
        .find(|(_, record)| record.storage_key == identifier)
        .map(|(key, record)| (key.clone(), record.clone()))
    {
        let link_name = record_link_name(&record_key, &record);
        return Ok((
            resolve_local_library_path(main_dir, &record_key, &record),
            record_key,
            link_name,
        ));
    }

    validate_skill_dir_name(identifier)?;
    Ok((
        main_dir.join(identifier),
        identifier.to_string(),
        identifier.to_string(),
    ))
}

fn installation_matches_delete(
    installation: &Installation,
    source_path: &std::path::Path,
    record_key: &str,
    _link_name: &str,
    identifier: &str,
) -> bool {
    if !installation.skill_storage_key.is_empty()
        && (installation.skill_storage_key == record_key
            || installation.skill_storage_key == identifier)
    {
        return true;
    }

    crate::link_installer::same_path(&installation.source_path, source_path)
}

fn cleanup_skill_hub_metadata(config: &mut AppConfig, record_key: &str) {
    config.skill_records.remove(record_key);
    config.skill_update_cache.updates.retain(|update| {
        update.storage_key != record_key
    });
    config
        .skill_discover_cache
        .skills
        .retain(|skill| skill.storage_key != record_key);
}

fn validate_skill_identifier(identifier: &str) -> Result<(), AppError> {
    if identifier.is_empty() {
        return Err(AppError::InvalidSkill {
            skill_dir_name: identifier.to_string(),
            message: "skill identifier must not be empty".to_string(),
        });
    }
    if identifier.contains("..") {
        return Err(AppError::InvalidSkill {
            skill_dir_name: identifier.to_string(),
            message: "skill identifier must not contain '..'".to_string(),
        });
    }
    if identifier.contains('\\') {
        return Err(AppError::InvalidSkill {
            skill_dir_name: identifier.to_string(),
            message: "skill identifier must not contain path separators".to_string(),
        });
    }
    if !identifier.contains('/') {
        validate_skill_dir_name(identifier)?;
    }
    Ok(())
}

fn validate_skill_dir_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::InvalidSkill {
            skill_dir_name: name.to_string(),
            message: "skill directory name must not be empty".to_string(),
        });
    }
    if name == "." || name == ".." {
        return Err(AppError::InvalidSkill {
            skill_dir_name: name.to_string(),
            message: "skill directory name must not be '.' or '..'".to_string(),
        });
    }
    if name.contains('/') || name.contains('\\') {
        return Err(AppError::InvalidSkill {
            skill_dir_name: name.to_string(),
            message: "skill directory name must not contain path separators".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppConfig, Installation, SkillView, Target};
    use crate::models::{
        DiscoverableSkill, SkillDiscoverCache, SkillRecord, SkillUpdateCache, SkillUpdateInfo,
        default_github_host,
    };
    use std::fs;
    use std::path::Path;

    fn create_valid_skill(main_dir: &Path, dir_name: &str) -> SkillView {
        let skill_dir = main_dir.join(dir_name);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: Test skill.\n---\n\n# Skill\n",
                dir_name
            ),
        )
        .expect("write skill md");
        SkillView {
            dir_name: dir_name.to_string(),
            name: Some(dir_name.to_string()),
            description: Some("Test skill.".to_string()),
            path: skill_dir,
            valid: true,
            validation_errors: Vec::new(),
            link_name: dir_name.to_string(),
            ..Default::default()
        }
    }

    fn create_nested_valid_skill(main_dir: &Path, storage_key: &str, link_name: &str) -> SkillView {
        let skill_dir = skill_storage::main_library_path(main_dir, storage_key);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: Test skill.\n---\n\n# Skill\n",
                link_name
            ),
        )
        .expect("write skill md");
        SkillView {
            dir_name: link_name.to_string(),
            name: Some(link_name.to_string()),
            description: Some("Test skill.".to_string()),
            path: skill_dir,
            valid: true,
            validation_errors: Vec::new(),
            storage_key: storage_key.to_string(),
            link_name: link_name.to_string(),
            ..Default::default()
        }
    }

    fn create_config_with_main_dir(temp: &Path) -> (AppConfig, std::path::PathBuf) {
        let main_dir = temp.join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let mut config = AppConfig::default();
        config.settings.main_skills_dir = Some(main_dir.clone());
        (config, main_dir)
    }

    #[test]
    fn rejects_when_confirmation_is_false() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, main_dir) = create_config_with_main_dir(temp.path());
        create_valid_skill(&main_dir, "brainstorming");

        let error = delete_main_skill(&mut config, "brainstorming", false)
            .expect_err("should reject without confirmation");

        assert!(
            matches!(error, AppError::Io { ref message, .. } if message.contains("需要确认"))
        );
    }

    #[test]
    fn deletes_uninstalled_source_skill() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, main_dir) = create_config_with_main_dir(temp.path());
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let result = delete_main_skill(&mut config, "brainstorming", true)
            .expect("should delete uninstalled skill");

        assert_eq!(result.deleted_skill_dir_name, "brainstorming");
        assert_eq!(result.removed_link_count, 0);
        assert!(!skill.path.exists());
        assert!(config.installations.is_empty());
    }

    #[test]
    fn deletes_nested_skill_by_storage_key() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, main_dir) = create_config_with_main_dir(temp.path());
        let storage_key = "repo/github.com--anthropics-skills/brainstorming";
        let skill = create_nested_valid_skill(&main_dir, storage_key, "brainstorming");
        config.skill_records.insert(
            storage_key.to_string(),
            SkillRecord {
                repo_host: default_github_host(),
                project_path: "anthropics/skills".to_string(),
                source: "github".to_string(),
                repo_owner: "anthropics".to_string(),
                repo_name: "skills".to_string(),
                repo_branch: "main".to_string(),
                directory: "skills/brainstorming".to_string(),
                content_hash: "hash".to_string(),
                installed_at: "2026-01-01T00:00:00Z".to_string(),
                storage_key: storage_key.to_string(),
                link_name: "brainstorming".to_string(),
                repo_slug: "github.com--anthropics-skills".to_string(),
                ..Default::default()
            },
        );

        let result = delete_main_skill(&mut config, storage_key, true).expect("delete nested skill");

        assert_eq!(result.deleted_skill_dir_name, "brainstorming");
        assert!(!skill.path.exists());
        assert!(!config.skill_records.contains_key(storage_key));
        assert!(
            !main_dir.join("repo").exists(),
            "empty storage namespace parents should be pruned after deleting the last skill"
        );
    }

    #[test]
    fn rejects_delete_by_link_name_alone_for_nested_skill() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, main_dir) = create_config_with_main_dir(temp.path());
        let storage_key = "repo/github.com--anthropics-skills/brainstorming";
        let skill = create_nested_valid_skill(&main_dir, storage_key, "brainstorming");
        config.skill_records.insert(
            storage_key.to_string(),
            SkillRecord {
                repo_host: default_github_host(),
                project_path: "anthropics/skills".to_string(),
                source: "github".to_string(),
                repo_owner: "anthropics".to_string(),
                repo_name: "skills".to_string(),
                repo_branch: "main".to_string(),
                directory: "skills/brainstorming".to_string(),
                content_hash: "hash".to_string(),
                installed_at: "2026-01-01T00:00:00Z".to_string(),
                storage_key: storage_key.to_string(),
                link_name: "brainstorming".to_string(),
                repo_slug: "github.com--anthropics-skills".to_string(),
                ..Default::default()
            },
        );

        let error = delete_main_skill(&mut config, "brainstorming", true)
            .expect_err("link_name alone must not resolve nested skills");

        assert!(matches!(error, AppError::Io { .. }));
        assert!(skill.path.exists());
        assert!(config.skill_records.contains_key(storage_key));
    }

    #[test]
    fn cleans_multiple_recorded_links_before_deleting_source_skill() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, main_dir) = create_config_with_main_dir(temp.path());
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target1_dir = temp.path().join("target-1");
        fs::create_dir_all(&target1_dir).expect("create target1 dir");
        let target2_dir = temp.path().join("target-2");
        fs::create_dir_all(&target2_dir).expect("create target2 dir");

        let link1 = target1_dir.join("brainstorming");
        let link2 = target2_dir.join("brainstorming");

        crate::fs_adapter::create_dir_link(&skill.path, &link1, crate::fs_adapter::default_link_type())
            .expect("create link1");
        crate::fs_adapter::create_dir_link(&skill.path, &link2, crate::fs_adapter::default_link_type())
            .expect("create link2");

        config.targets = vec![
            Target::global_custom("target-1", "Target One", target1_dir.clone(), "1", "1"),
            Target::global_custom("target-2", "Target Two", target2_dir.clone(), "1", "1"),
        ];

        config.installations = vec![
            Installation {
                id: "install-1".to_string(),
                skill_dir_name: "brainstorming".to_string(),
                skill_name: "brainstorming".to_string(),
                source_path: skill.path.clone(),
                target_id: "target-1".to_string(),
                link_path: link1.clone(),
                link_type: crate::fs_adapter::default_link_type(),
                created_at: "1".to_string(),
                ..Default::default()
            },
            Installation {
                id: "install-2".to_string(),
                skill_dir_name: "brainstorming".to_string(),
                skill_name: "brainstorming".to_string(),
                source_path: skill.path.clone(),
                target_id: "target-2".to_string(),
                link_path: link2.clone(),
                link_type: crate::fs_adapter::default_link_type(),
                created_at: "1".to_string(),
                ..Default::default()
            },
        ];

        let result = delete_main_skill(&mut config, "brainstorming", true)
            .expect("should delete with links");

        assert_eq!(result.deleted_skill_dir_name, "brainstorming");
        assert_eq!(result.removed_link_count, 2);
        assert!(!skill.path.exists());
        assert!(!crate::fs_adapter::path_exists(&link1));
        assert!(!crate::fs_adapter::path_exists(&link2));
    }

    #[test]
    fn aborts_source_deletion_if_one_recorded_link_cleanup_fails() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, main_dir) = create_config_with_main_dir(temp.path());
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target1_dir = temp.path().join("target-1");
        fs::create_dir_all(&target1_dir).expect("create target1 dir");
        let target2_dir = temp.path().join("target-2");
        fs::create_dir_all(&target2_dir).expect("create target2 dir");

        let link1 = target1_dir.join("brainstorming");
        let link2 = target2_dir.join("brainstorming");

        crate::fs_adapter::create_dir_link(&skill.path, &link1, crate::fs_adapter::default_link_type())
            .expect("create link1");
        crate::fs_adapter::create_dir_link(&skill.path, &link2, crate::fs_adapter::default_link_type())
            .expect("create link2");

        // Corrupt link2 by replacing it with a real directory so remove_recorded_link fails
        crate::fs_adapter::remove_recorded_link(&link2, &skill.path).expect("remove link2");
        fs::create_dir_all(&link2).expect("create real dir at link2");
        fs::write(link2.join("keep.txt"), "keep").expect("write file");

        config.targets = vec![
            Target::global_custom("target-1", "Target One", target1_dir.clone(), "1", "1"),
            Target::global_custom("target-2", "Target Two", target2_dir.clone(), "1", "1"),
        ];

        config.installations = vec![
            Installation {
                id: "install-1".to_string(),
                skill_dir_name: "brainstorming".to_string(),
                skill_name: "brainstorming".to_string(),
                source_path: skill.path.clone(),
                target_id: "target-1".to_string(),
                link_path: link1.clone(),
                link_type: crate::fs_adapter::default_link_type(),
                created_at: "1".to_string(),
                ..Default::default()
            },
            Installation {
                id: "install-2".to_string(),
                skill_dir_name: "brainstorming".to_string(),
                skill_name: "brainstorming".to_string(),
                source_path: skill.path.clone(),
                target_id: "target-2".to_string(),
                link_path: link2.clone(),
                link_type: crate::fs_adapter::default_link_type(),
                created_at: "1".to_string(),
                ..Default::default()
            },
        ];

        let error = delete_main_skill(&mut config, "brainstorming", true)
            .expect_err("should fail when link cleanup fails");

        assert!(matches!(error, AppError::Io { .. }));
        // Source skill should still exist
        assert!(skill.path.exists());
        assert!(skill.path.join("SKILL.md").is_file());
        // Installation records should still be present
        assert_eq!(config.installations.len(), 2);
    }

    #[test]
    fn deletes_skill_and_record_even_when_link_target_mismatches() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, main_dir) = create_config_with_main_dir(temp.path());
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target_dir = temp.path().join("target-1");
        fs::create_dir_all(&target_dir).expect("create target dir");
        let link = target_dir.join("brainstorming");
        let other_source = temp.path().join("other-source");
        fs::create_dir_all(&other_source).expect("create other source");
        crate::fs_adapter::create_dir_link(
            &other_source,
            &link,
            crate::fs_adapter::default_link_type(),
        )
        .expect("create mismatch link");

        config.targets = vec![Target::global_custom(
            "target-1",
            "Target One",
            target_dir.clone(),
            "1",
            "1",
        )];
        config.installations = vec![Installation {
            id: "install-1".to_string(),
            skill_dir_name: "brainstorming".to_string(),
            skill_name: "brainstorming".to_string(),
            source_path: skill.path.clone(),
            target_id: "target-1".to_string(),
            link_path: link.clone(),
            link_type: crate::fs_adapter::default_link_type(),
            created_at: "1".to_string(),
            ..Default::default()
        }];

        let result = delete_main_skill(&mut config, "brainstorming", true)
            .expect("mismatch link should not block source delete");

        assert_eq!(result.removed_link_count, 1);
        assert!(!skill.path.exists());
        // Drifted link is left for the user; record is still cleared.
        assert!(crate::fs_adapter::path_exists(&link));
        assert!(other_source.exists());
        assert!(config.installations.is_empty());
    }

    #[test]
    fn successful_deletion_removes_related_installation_records() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, main_dir) = create_config_with_main_dir(temp.path());
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target1_dir = temp.path().join("target-1");
        fs::create_dir_all(&target1_dir).expect("create target1 dir");

        let link1 = target1_dir.join("brainstorming");

        crate::fs_adapter::create_dir_link(&skill.path, &link1, crate::fs_adapter::default_link_type())
            .expect("create link1");

        config.targets = vec![Target::global_custom(
            "target-1",
            "Target One",
            target1_dir.clone(),
            "1",
            "1",
        )];

        config.installations = vec![
            Installation {
                id: "install-1".to_string(),
                skill_dir_name: "brainstorming".to_string(),
                skill_name: "brainstorming".to_string(),
                source_path: skill.path.clone(),
                target_id: "target-1".to_string(),
                link_path: link1.clone(),
                link_type: crate::fs_adapter::default_link_type(),
                created_at: "1".to_string(),
                ..Default::default()
            },
            Installation {
                id: "install-2".to_string(),
                skill_dir_name: "other-skill".to_string(),
                skill_name: "other-skill".to_string(),
                source_path: main_dir.join("other-skill"),
                target_id: "target-1".to_string(),
                link_path: target1_dir.join("other-skill"),
                link_type: crate::fs_adapter::default_link_type(),
                created_at: "1".to_string(),
                ..Default::default()
            },
        ];

        let result = delete_main_skill(&mut config, "brainstorming", true)
            .expect("should delete and remove records");

        assert_eq!(result.deleted_skill_dir_name, "brainstorming");
        assert_eq!(result.removed_link_count, 1);
        assert!(!skill.path.exists());
        assert!(!crate::fs_adapter::path_exists(&link1));
        // Only the unrelated installation record should remain
        assert_eq!(config.installations.len(), 1);
        assert_eq!(config.installations[0].skill_dir_name, "other-skill");
    }

    #[test]
    fn successful_deletion_cleans_skill_hub_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, main_dir) = create_config_with_main_dir(temp.path());
        create_valid_skill(&main_dir, "brainstorming");

        config.skill_records.insert(
            "brainstorming".to_string(),
            SkillRecord {
                repo_host: default_github_host(),
                project_path: "owner/repo".to_string(),
                source: "github".to_string(),
                repo_owner: "owner".to_string(),
                repo_name: "repo".to_string(),
                repo_branch: "main".to_string(),
                directory: "skills/brainstorming".to_string(),
                content_hash: "hash".to_string(),
                installed_at: "2026-01-01T00:00:00Z".to_string(),
                storage_key: "brainstorming".to_string(),
                link_name: "brainstorming".to_string(),
                ..Default::default()
            },
        );
        config.skill_update_cache = SkillUpdateCache {
            checked_at: Some("2026-01-01T00:00:00Z".to_string()),
            updates: vec![
                SkillUpdateInfo {
                    dir_name: "brainstorming".to_string(),
                    name: "brainstorming".to_string(),
                    current_hash: Some("old".to_string()),
                    remote_hash: "new".to_string(),
                    storage_key: "brainstorming".to_string(),
                    ..Default::default()
                },
                SkillUpdateInfo {
                    dir_name: "other-skill".to_string(),
                    name: "other-skill".to_string(),
                    current_hash: Some("a".to_string()),
                    remote_hash: "b".to_string(),
                    storage_key: "other-skill".to_string(),
                    ..Default::default()
                },
            ],
        };
        config.skill_discover_cache = SkillDiscoverCache {
            fetched_at: Some("2026-01-01T00:00:00Z".to_string()),
            skills: vec![
                DiscoverableSkill {
                    key: "owner/repo/skills/brainstorming".to_string(),
                    name: "brainstorming".to_string(),
                    description: "Test".to_string(),
                    directory: "skills/brainstorming".to_string(),
                    install_dir_name: "brainstorming".to_string(),
                    storage_key: "brainstorming".to_string(),
                    repo_host: default_github_host(),
                    project_path: "owner/repo".to_string(),
                    repo_owner: "owner".to_string(),
                    repo_name: "repo".to_string(),
                    repo_branch: "main".to_string(),
                    source: "github".to_string(),
                    ..Default::default()
                },
                DiscoverableSkill {
                    key: "owner/repo/skills/other".to_string(),
                    name: "other".to_string(),
                    description: "Other".to_string(),
                    directory: "skills/other".to_string(),
                    install_dir_name: "other-skill".to_string(),
                    storage_key: "other-skill".to_string(),
                    repo_host: default_github_host(),
                    project_path: "owner/repo".to_string(),
                    repo_owner: "owner".to_string(),
                    repo_name: "repo".to_string(),
                    repo_branch: "main".to_string(),
                    source: "github".to_string(),
                    ..Default::default()
                },
            ],
        };

        delete_main_skill(&mut config, "brainstorming", true).expect("delete skill");

        assert!(!config.skill_records.contains_key("brainstorming"));
        assert_eq!(config.skill_update_cache.updates.len(), 1);
        assert_eq!(config.skill_update_cache.updates[0].dir_name, "other-skill");
        assert_eq!(config.skill_discover_cache.skills.len(), 1);
        assert_eq!(
            config.skill_discover_cache.skills[0].install_dir_name,
            "other-skill"
        );
    }

    #[test]
    fn rejects_invalid_skill_dir_names() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, _main_dir) = create_config_with_main_dir(temp.path());

        let invalid_names = vec!["", ".", "..", "foo\\bar", "../escape"];
        for name in invalid_names {
            let error = delete_main_skill(&mut config, name, true)
                .expect_err(&format!("should reject invalid name '{}'", name));
            assert!(
                matches!(error, AppError::InvalidSkill { ref skill_dir_name, .. } if skill_dir_name == name),
                "expected InvalidSkill for name '{}', got {:?}",
                name,
                error
            );
        }
    }
}
