use crate::models::{AppConfig, AppError, DeleteMainSkillResult, Installation};
use std::path::Path;

pub fn delete_main_skill(
    config: &mut AppConfig,
    skill_dir_name: &str,
    confirmed: bool,
) -> Result<DeleteMainSkillResult, AppError> {
    if !confirmed {
        return Err(AppError::Io {
            path: None,
            message: "main skill deletion requires confirmation".to_string(),
        });
    }

    let main_dir = config
        .settings
        .main_skills_dir
        .as_ref()
        .ok_or_else(|| AppError::Io {
            path: None,
            message: "main skills directory is not configured".to_string(),
        })?;

    let source_path = main_dir.join(skill_dir_name);

    if !source_path.is_dir() {
        return Err(AppError::Io {
            path: Some(source_path.clone()),
            message: format!("source skill directory '{}' does not exist", skill_dir_name),
        });
    }

    let related: Vec<Installation> = config
        .installations
        .iter()
        .filter(|i| {
            i.skill_dir_name == skill_dir_name
                || i.source_path == source_path
        })
        .cloned()
        .collect();

    for installation in &related {
        crate::fs_adapter::remove_recorded_link(
            &installation.link_path,
            &installation.source_path,
        )?;
    }

    crate::fs_adapter::delete_real_dir(&source_path)?;

    config
        .installations
        .retain(|i| {
            i.skill_dir_name != skill_dir_name && i.source_path != source_path
        });

    Ok(DeleteMainSkillResult {
        deleted_skill_dir_name: skill_dir_name.to_string(),
        removed_link_count: related.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppConfig, Installation, Settings, SkillView, Target};
    use std::fs;
    use std::path::Path;

    fn create_target_config(
        temp: &Path,
        target_id: &str,
        target_name: &str,
    ) -> (AppConfig, std::path::PathBuf) {
        let target_dir = temp.join(format!("target-{}", target_id));
        fs::create_dir_all(&target_dir).expect("create target dir");
        let config = AppConfig {
            version: 1,
            settings: Settings::default(),
            targets: vec![Target {
                id: target_id.to_string(),
                name: target_name.to_string(),
                skills_dir: target_dir.clone(),
                created_at: "1".to_string(),
                updated_at: "1".to_string(),
            }],
            installations: Vec::new(),
        };
        (config, target_dir)
    }

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
            matches!(error, AppError::Io { ref message, .. } if message.contains("requires confirmation"))
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
            Target {
                id: "target-1".to_string(),
                name: "Target One".to_string(),
                skills_dir: target1_dir.clone(),
                created_at: "1".to_string(),
                updated_at: "1".to_string(),
            },
            Target {
                id: "target-2".to_string(),
                name: "Target Two".to_string(),
                skills_dir: target2_dir.clone(),
                created_at: "1".to_string(),
                updated_at: "1".to_string(),
            },
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
            Target {
                id: "target-1".to_string(),
                name: "Target One".to_string(),
                skills_dir: target1_dir.clone(),
                created_at: "1".to_string(),
                updated_at: "1".to_string(),
            },
            Target {
                id: "target-2".to_string(),
                name: "Target Two".to_string(),
                skills_dir: target2_dir.clone(),
                created_at: "1".to_string(),
                updated_at: "1".to_string(),
            },
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
    fn successful_deletion_removes_related_installation_records() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, main_dir) = create_config_with_main_dir(temp.path());
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target1_dir = temp.path().join("target-1");
        fs::create_dir_all(&target1_dir).expect("create target1 dir");

        let link1 = target1_dir.join("brainstorming");

        crate::fs_adapter::create_dir_link(&skill.path, &link1, crate::fs_adapter::default_link_type())
            .expect("create link1");

        config.targets = vec![Target {
            id: "target-1".to_string(),
            name: "Target One".to_string(),
            skills_dir: target1_dir.clone(),
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        }];

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
}
