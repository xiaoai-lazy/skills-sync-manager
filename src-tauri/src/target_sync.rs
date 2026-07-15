use crate::link_installer;
use crate::models::{AppConfig, AppError, SkillView, SyncInstallFailure, TargetScope};
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct SyncCounts {
    pub installed: u32,
    pub skipped: u32,
    pub failed: Vec<SyncInstallFailure>,
}

pub fn sync_target_installations(
    config: &mut AppConfig,
    source_target_id: &str,
    dest_target_id: &str,
    skills: &[SkillView],
) -> Result<SyncCounts, AppError> {
    if source_target_id == dest_target_id {
        return Err(AppError::Io {
            path: None,
            message: "不能从同一目标同步".to_string(),
        });
    }

    let source = config
        .targets
        .iter()
        .find(|t| t.id == source_target_id)
        .cloned()
        .ok_or_else(|| AppError::TargetNotFound {
            target_id: source_target_id.to_string(),
        })?;
    let dest = config
        .targets
        .iter()
        .find(|t| t.id == dest_target_id)
        .cloned()
        .ok_or_else(|| AppError::TargetNotFound {
            target_id: dest_target_id.to_string(),
        })?;

    if source.scope != TargetScope::Project || dest.scope != TargetScope::Project {
        return Err(AppError::Io {
            path: None,
            message: "只能同步项目级目标目录".to_string(),
        });
    }
    if source.project_id.is_none() || source.project_id != dest.project_id {
        return Err(AppError::Io {
            path: None,
            message: "只能同步同一项目下的目标目录".to_string(),
        });
    }

    let keys: BTreeSet<String> = config
        .installations
        .iter()
        .filter(|i| i.target_id == source_target_id)
        .map(|i| i.skill_storage_key.clone())
        .collect();

    let mut installed = 0u32;
    let mut skipped = 0u32;
    let mut failed = Vec::new();

    for storage_key in keys {
        let label = skills
            .iter()
            .find(|s| s.storage_key == storage_key)
            .and_then(|s| s.name.clone().or_else(|| Some(s.dir_name.clone())))
            .or_else(|| {
                config
                    .installations
                    .iter()
                    .find(|i| {
                        i.target_id == source_target_id && i.skill_storage_key == storage_key
                    })
                    .map(|i| i.skill_name.clone())
            })
            .unwrap_or_else(|| storage_key.clone());

        // Pre-check idempotent install → skipped (install_skill also returns Ok for noop)
        if link_installer::is_skill_installed(config, dest_target_id, &storage_key, skills) {
            skipped += 1;
            continue;
        }

        match link_installer::install_skill(config, dest_target_id, &storage_key, skills) {
            Ok(()) => installed += 1,
            Err(AppError::Conflict { message, .. }) => {
                skipped += 1;
                let _ = message;
            }
            Err(err) => {
                failed.push(SyncInstallFailure {
                    storage_key,
                    label,
                    error: err.to_dto().message,
                });
            }
        }
    }

    Ok(SyncCounts {
        installed,
        skipped,
        failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_adapter;
    use crate::link_installer;
    use crate::models::{Installation, Settings, Target, TargetKind, TargetScope};
    use std::fs;
    use std::path::{Path, PathBuf};

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
            storage_key: dir_name.to_string(),
            link_name: dir_name.to_string(),
            ..Default::default()
        }
    }

    fn project_target(
        id: &str,
        name: &str,
        skills_dir: PathBuf,
        project_id: &str,
    ) -> Target {
        Target {
            id: id.to_string(),
            name: name.to_string(),
            scope: TargetScope::Project,
            kind: TargetKind::Custom,
            agent_id: None,
            project_id: Some(project_id.to_string()),
            custom_path: Some(skills_dir.clone()),
            skills_dir,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        }
    }

    fn two_project_targets(
        temp: &Path,
        project_id: &str,
    ) -> (AppConfig, PathBuf, PathBuf) {
        let source_dir = temp.join("target-source");
        let dest_dir = temp.join("target-dest");
        fs::create_dir_all(&source_dir).expect("create source dir");
        fs::create_dir_all(&dest_dir).expect("create dest dir");
        let config = AppConfig {
            version: 1,
            settings: Settings::default(),
            targets: vec![
                project_target("source", "Source", source_dir.clone(), project_id),
                project_target("dest", "Dest", dest_dir.clone(), project_id),
            ],
            installations: Vec::new(),
            ..Default::default()
        };
        (config, source_dir, dest_dir)
    }

    #[test]
    fn sync_installs_missing_skills_and_skips_already_installed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill_a = create_valid_skill(&main_dir, "skill-a");
        let skill_b = create_valid_skill(&main_dir, "skill-b");
        let skills = vec![skill_a.clone(), skill_b.clone()];

        let (mut config, _source_dir, dest_dir) = two_project_targets(temp.path(), "project-1");

        link_installer::install_skill(&mut config, "source", "skill-a", &skills)
            .expect("install A on source");
        link_installer::install_skill(&mut config, "source", "skill-b", &skills)
            .expect("install B on source");
        link_installer::install_skill(&mut config, "dest", "skill-a", &skills)
            .expect("install A on dest");

        let counts = sync_target_installations(&mut config, "source", "dest", &skills)
            .expect("sync should succeed");

        assert_eq!(counts.installed, 1);
        assert!(counts.skipped >= 1);
        assert!(counts.failed.is_empty());
        assert!(fs_adapter::path_exists(&dest_dir.join("skill-b")));
        assert!(config.installations.iter().any(|i| {
            i.target_id == "dest" && i.skill_storage_key == "skill-b"
        }));
    }

    #[test]
    fn sync_rejects_different_project_ids() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source_dir = temp.path().join("target-source");
        let dest_dir = temp.path().join("target-dest");
        fs::create_dir_all(&source_dir).expect("create source dir");
        fs::create_dir_all(&dest_dir).expect("create dest dir");
        let mut config = AppConfig {
            version: 1,
            settings: Settings::default(),
            targets: vec![
                project_target("source", "Source", source_dir, "project-1"),
                project_target("dest", "Dest", dest_dir, "project-2"),
            ],
            installations: Vec::new(),
            ..Default::default()
        };

        let error = sync_target_installations(&mut config, "source", "dest", &[])
            .expect_err("different projects should fail");
        assert!(matches!(error, AppError::Io { .. }));
    }

    #[test]
    fn sync_rejects_same_target_id() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, _, _) = two_project_targets(temp.path(), "project-1");

        let error = sync_target_installations(&mut config, "source", "source", &[])
            .expect_err("same target should fail");
        assert!(matches!(error, AppError::Io { .. }));
    }

    #[test]
    fn sync_rejects_global_scope() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source_dir = temp.path().join("target-source");
        let dest_dir = temp.path().join("target-dest");
        fs::create_dir_all(&source_dir).expect("create source dir");
        fs::create_dir_all(&dest_dir).expect("create dest dir");
        let mut config = AppConfig {
            version: 1,
            settings: Settings::default(),
            targets: vec![
                Target::global_custom("source", "Source", source_dir, "1", "1"),
                project_target("dest", "Dest", dest_dir, "project-1"),
            ],
            installations: Vec::new(),
            ..Default::default()
        };

        let error = sync_target_installations(&mut config, "source", "dest", &[])
            .expect_err("global scope should fail");
        assert!(matches!(error, AppError::Io { .. }));
    }

    #[test]
    fn sync_reports_failed_for_missing_main_library_skill() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (mut config, source_dir, _) = two_project_targets(temp.path(), "project-1");
        let missing_key = "missing-skill";
        let link_path = source_dir.join(missing_key);
        config.installations.push(Installation {
            id: "install-missing".to_string(),
            skill_dir_name: missing_key.to_string(),
            skill_name: "Missing Skill".to_string(),
            source_path: temp.path().join("gone"),
            target_id: "source".to_string(),
            link_path,
            link_type: crate::models::LinkType::Junction,
            created_at: "1".to_string(),
            skill_storage_key: missing_key.to_string(),
        });

        let counts = sync_target_installations(&mut config, "source", "dest", &[])
            .expect("sync should classify missing skill as failed");

        assert_eq!(counts.installed, 0);
        assert!(!counts.failed.is_empty());
        assert_eq!(counts.failed[0].storage_key, missing_key);
        assert!(!counts.failed[0].error.is_empty());
    }
}
