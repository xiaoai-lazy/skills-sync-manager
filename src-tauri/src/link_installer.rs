use crate::fs_adapter;
use crate::models::{
    AppConfig, AppError, Installation, SkillInstallState, SkillView, SkillWithTargetState, Target,
};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn install_skill(
    config: &mut AppConfig,
    target_id: &str,
    skill_dir_name: &str,
    skills: &[SkillView],
) -> Result<(), AppError> {
    let target = find_target(config, target_id)?;
    let skill = find_valid_skill(skills, skill_dir_name)?;

    crate::target_registry::validate_target_dir(&target.skills_dir)?;

    let link_path = target.skills_dir.join(&skill.dir_name);

    if let Some(installation) = find_installation(config, target_id, skill_dir_name) {
        if link_matches_record(&link_path, installation) {
            return Ok(());
        }
    }

    if fs_adapter::path_exists(&link_path) {
        return Err(AppError::Conflict {
            path: link_path,
            message: "目标路径已存在同名内容，无法安装".to_string(),
        });
    }

    fs_adapter::create_dir_link(&skill.path, &link_path, fs_adapter::default_link_type())?;

    let installation = Installation {
        id: format!("install-{}", timestamp_nanos()),
        skill_dir_name: skill.dir_name.clone(),
        skill_name: skill.name.clone().unwrap_or_else(|| skill.dir_name.clone()),
        source_path: skill.path.clone(),
        target_id: target_id.to_string(),
        link_path,
        link_type: fs_adapter::default_link_type(),
        created_at: current_timestamp(),
    };
    config.installations.push(installation);

    Ok(())
}

pub fn uninstall_skill(
    config: &mut AppConfig,
    target_id: &str,
    skill_dir_name: &str,
) -> Result<(), AppError> {
    let installation =
        find_installation(config, target_id, skill_dir_name).ok_or_else(|| AppError::Io {
            path: None,
            message: format!(
                "no installation record found for target '{}' and skill '{}'",
                target_id, skill_dir_name
            ),
        })?;

    fs_adapter::remove_recorded_link(&installation.link_path, &installation.source_path)?;

    config
        .installations
        .retain(|i| i.target_id != target_id || i.skill_dir_name != skill_dir_name);

    Ok(())
}

pub fn compute_target_skill_states(
    config: &AppConfig,
    target_id: &str,
    skills: &[SkillView],
) -> Result<Vec<SkillWithTargetState>, AppError> {
    let target = find_target(config, target_id)?;

    let skill_dir_names: std::collections::HashSet<&str> =
        skills.iter().map(|s| s.dir_name.as_str()).collect();

    let mut states = Vec::new();

    // Compute states for skills currently in the library
    for skill in skills {
        let state = compute_skill_state(config, target, skill);
        states.push(state);
    }

    // Add sourceMissing states for installation records whose skill is no longer in the library
    for installation in config
        .installations
        .iter()
        .filter(|i| i.target_id == target_id)
    {
        if !skill_dir_names.contains(installation.skill_dir_name.as_str()) {
            states.push(SkillWithTargetState {
                skill: SkillView {
                    dir_name: installation.skill_dir_name.clone(),
                    name: Some(installation.skill_name.clone()),
                    description: None,
                    path: installation.source_path.clone(),
                    valid: false,
                    validation_errors: vec!["源 skill 已不存在".to_string()],
                },
                state: SkillInstallState::SourceMissing,
                message: Some(
                    "安装记录存在，但源 skill 已不在库中"
                        .to_string(),
                ),
            });
        }
    }

    Ok(states)
}

pub fn find_installation<'a>(
    config: &'a AppConfig,
    target_id: &str,
    skill_dir_name: &str,
) -> Option<&'a Installation> {
    config.installations.iter().find(|installation| {
        installation.target_id == target_id && installation.skill_dir_name == skill_dir_name
    })
}

fn find_target<'a>(config: &'a AppConfig, target_id: &str) -> Result<&'a Target, AppError> {
    config
        .targets
        .iter()
        .find(|target| target.id == target_id)
        .ok_or_else(|| AppError::TargetNotFound {
            target_id: target_id.to_string(),
        })
}

fn find_valid_skill<'a>(
    skills: &'a [SkillView],
    skill_dir_name: &str,
) -> Result<&'a SkillView, AppError> {
    let skill = skills
        .iter()
        .find(|skill| skill.dir_name == skill_dir_name)
        .ok_or_else(|| AppError::InvalidSkill {
            skill_dir_name: skill_dir_name.to_string(),
            message: "skill not found in library".to_string(),
        })?;

    if !skill.valid {
        return Err(AppError::InvalidSkill {
            skill_dir_name: skill_dir_name.to_string(),
            message: format!("skill validation failed: {:?}", skill.validation_errors),
        });
    }

    Ok(skill)
}

fn link_matches_record(link_path: &Path, installation: &Installation) -> bool {
    if !fs_adapter::path_exists(link_path) {
        return false;
    }

    match fs_adapter::link_target(link_path) {
        Ok(Some(actual_target)) => same_path(&actual_target, &installation.source_path),
        _ => false,
    }
}

pub fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left_canon), Ok(right_canon)) => left_canon == right_canon,
        _ => left == right,
    }
}

fn compute_skill_state(
    config: &AppConfig,
    target: &Target,
    skill: &SkillView,
) -> SkillWithTargetState {
    if !skill.valid {
        return SkillWithTargetState {
            skill: skill.clone(),
            state: SkillInstallState::InvalidSkill,
            message: Some(format!(
                "无效 skill：{}",
                skill.validation_errors.join(", ")
            )),
        };
    }

    let link_path = target.skills_dir.join(&skill.dir_name);

    if let Some(installation) = find_installation(config, &target.id, &skill.dir_name) {
        if !fs_adapter::path_exists(&skill.path) {
            return SkillWithTargetState {
                skill: skill.clone(),
                state: SkillInstallState::SourceMissing,
                message: Some(
                    "安装记录存在，但源 skill 目录已缺失".to_string(),
                ),
            };
        }

        if !fs_adapter::path_exists(&link_path) {
            return SkillWithTargetState {
                skill: skill.clone(),
                state: SkillInstallState::Missing,
                message: Some("安装记录存在，但链接已不存在".to_string()),
            };
        }

        match fs_adapter::link_target(&link_path) {
            Ok(Some(actual_target)) => {
                if same_path(&actual_target, &installation.source_path) {
                    SkillWithTargetState {
                        skill: skill.clone(),
                        state: SkillInstallState::Installed,
                        message: Some("Skill 已安装".to_string()),
                    }
                } else {
                    SkillWithTargetState {
                        skill: skill.clone(),
                        state: SkillInstallState::Mismatch,
                        message: Some(format!(
                            "链接目标与记录不符：{} 指向 {}，但记录为 {}",
                            link_path.display(),
                            actual_target.display(),
                            installation.source_path.display()
                        )),
                    }
                }
            }
            Ok(None) => SkillWithTargetState {
                skill: skill.clone(),
                state: SkillInstallState::Mismatch,
                message: Some("路径存在，但不是链接".to_string()),
            },
            Err(err) => SkillWithTargetState {
                skill: skill.clone(),
                state: SkillInstallState::Mismatch,
                message: Some(format!("无法解析链接目标：{}", err)),
            },
        }
    } else {
        if fs_adapter::path_exists(&link_path) {
            SkillWithTargetState {
                skill: skill.clone(),
                state: SkillInstallState::Conflict,
                message: Some(format!(
                    "目标路径已存在同名内容：{}",
                    link_path.display()
                )),
            }
        } else {
            SkillWithTargetState {
                skill: skill.clone(),
                state: SkillInstallState::NotInstalled,
                message: Some("Skill 未安装".to_string()),
            }
        }
    }
}

fn current_timestamp() -> String {
    timestamp_nanos().to_string()
}

fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Settings;
    use std::fs;
    use std::path::PathBuf;

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

    fn create_invalid_skill(main_dir: &Path, dir_name: &str) -> SkillView {
        let skill_dir = main_dir.join(dir_name);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        SkillView {
            dir_name: dir_name.to_string(),
            name: None,
            description: None,
            path: skill_dir,
            valid: false,
            validation_errors: vec!["Missing SKILL.md".to_string()],
        }
    }

    fn create_target_config(
        temp: &Path,
        target_id: &str,
        target_name: &str,
    ) -> (AppConfig, PathBuf) {
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

    #[test]
    fn installing_valid_skill_creates_link_and_installation_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");
        let (mut config, target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect("install skill");

        let link_path = target_dir.join("brainstorming");
        assert!(fs_adapter::path_exists(&link_path));
        assert!(fs_adapter::is_dir(&link_path));
        assert_eq!(config.installations.len(), 1);
        assert_eq!(config.installations[0].skill_dir_name, "brainstorming");
        assert_eq!(config.installations[0].target_id, "target-1");
    }

    #[test]
    fn installing_invalid_skill_fails() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_invalid_skill(&main_dir, "broken-skill");
        let (mut config, _target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        let error = install_skill(&mut config, "target-1", "broken-skill", &[skill.clone()])
            .expect_err("invalid skill should fail");

        assert!(
            matches!(error, AppError::InvalidSkill { skill_dir_name, .. } if skill_dir_name == "broken-skill")
        );
        assert!(config.installations.is_empty());
    }

    #[test]
    fn target_same_name_real_directory_returns_conflict() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");
        let (mut config, target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        let existing_dir = target_dir.join("brainstorming");
        fs::create_dir_all(&existing_dir).expect("create existing dir");
        fs::write(existing_dir.join("existing.txt"), "existing").expect("write existing file");

        let error = install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect_err("existing dir should conflict");

        assert!(matches!(error, AppError::Conflict { .. }));
        assert!(existing_dir.join("existing.txt").is_file());
        assert!(config.installations.is_empty());
    }

    #[test]
    fn target_same_name_regular_file_returns_conflict() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");
        let (mut config, _target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        let existing_file = _target_dir.join("brainstorming");
        fs::write(&existing_file, "existing file content").expect("write existing file");

        let error = install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect_err("existing file should conflict");

        assert!(matches!(error, AppError::Conflict { .. }));
        assert!(existing_file.is_file());
        assert_eq!(
            fs::read_to_string(&existing_file).unwrap(),
            "existing file content"
        );
        assert!(config.installations.is_empty());
    }

    #[test]
    fn target_same_name_unknown_link_returns_conflict() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");
        let (mut config, target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        let other_source = temp.path().join("other-source");
        fs::create_dir_all(&other_source).expect("create other source");
        let unknown_link = target_dir.join("brainstorming");
        fs_adapter::create_dir_link(
            &other_source,
            &unknown_link,
            fs_adapter::default_link_type(),
        )
        .expect("create unknown link");

        let error = install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect_err("unknown link should conflict");

        assert!(matches!(error, AppError::Conflict { .. }));
        assert!(fs_adapter::path_exists(&unknown_link));
        assert!(config.installations.is_empty());
    }

    #[test]
    fn repeated_install_of_existing_recorded_correct_link_is_idempotent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");
        let (mut config, target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect("first install");
        let first_installation = config.installations[0].clone();

        install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect("second install should be idempotent");

        assert_eq!(config.installations.len(), 1);
        assert_eq!(config.installations[0], first_installation);
        let link_path = target_dir.join("brainstorming");
        assert!(fs_adapter::path_exists(&link_path));
    }

    #[test]
    fn uninstall_removes_recorded_link_and_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");
        let (mut config, target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect("install skill");
        let link_path = target_dir.join("brainstorming");
        assert!(fs_adapter::path_exists(&link_path));

        uninstall_skill(&mut config, "target-1", "brainstorming").expect("uninstall skill");

        assert!(!fs_adapter::path_exists(&link_path));
        assert!(config.installations.is_empty());
        assert!(skill.path.is_dir());
        assert!(skill.path.join("SKILL.md").is_file());
    }

    #[test]
    fn uninstall_does_not_delete_source_skill() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");
        let (mut config, target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect("install skill");

        uninstall_skill(&mut config, "target-1", "brainstorming").expect("uninstall skill");

        assert!(skill.path.is_dir());
        assert!(skill.path.join("SKILL.md").is_file());
        assert!(!fs_adapter::path_exists(&target_dir.join("brainstorming")));
    }

    #[test]
    fn uninstall_refuses_unknown_real_directory_at_link_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");
        let (mut config, target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect("install skill");

        let link_path = target_dir.join("brainstorming");
        fs_adapter::remove_recorded_link(&link_path, &skill.path).expect("remove link");
        fs::create_dir_all(&link_path).expect("create real dir at link path");
        fs::write(link_path.join("keep.txt"), "keep").expect("write file");

        let error = uninstall_skill(&mut config, "target-1", "brainstorming")
            .expect_err("real dir should fail");

        assert!(matches!(error, AppError::Io { .. }));
        assert!(link_path.is_dir());
        assert!(link_path.join("keep.txt").is_file());
        assert_eq!(config.installations.len(), 1);
    }

    #[test]
    fn uninstall_preserves_record_on_missing_link() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");
        let (mut config, target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect("install skill");

        let link_path = target_dir.join("brainstorming");
        fs_adapter::remove_recorded_link(&link_path, &skill.path).expect("remove link externally");
        assert!(!fs_adapter::path_exists(&link_path));

        let error = uninstall_skill(&mut config, "target-1", "brainstorming")
            .expect_err("missing link should fail");

        assert!(matches!(error, AppError::Io { .. }));
        assert_eq!(config.installations.len(), 1);
    }

    #[test]
    fn uninstall_preserves_record_on_mismatch_link() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");
        let (mut config, target_dir) = create_target_config(temp.path(), "target-1", "Target One");

        install_skill(&mut config, "target-1", "brainstorming", &[skill.clone()])
            .expect("install skill");

        let link_path = target_dir.join("brainstorming");
        fs_adapter::remove_recorded_link(&link_path, &skill.path).expect("remove original link");
        let other_source = temp.path().join("other-source");
        fs::create_dir_all(&other_source).expect("create other source");
        fs_adapter::create_dir_link(&other_source, &link_path, fs_adapter::default_link_type())
            .expect("create mismatch link");

        let error = uninstall_skill(&mut config, "target-1", "brainstorming")
            .expect_err("mismatch link should fail");

        assert!(matches!(error, AppError::Io { .. }));
        assert_eq!(config.installations.len(), 1);
    }

    #[test]
    fn compute_states_include_installed_not_installed_conflict_missing_mismatch_invalid() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let valid_skill = create_valid_skill(&main_dir, "valid-skill");
        let invalid_skill = create_invalid_skill(&main_dir, "invalid-skill");
        let uninstalled_skill = create_valid_skill(&main_dir, "uninstalled-skill");
        let conflict_skill = create_valid_skill(&main_dir, "conflict-skill");
        let missing_skill = create_valid_skill(&main_dir, "missing-skill");
        let mismatch_skill = create_valid_skill(&main_dir, "mismatch-skill");

        let target_dir = temp.path().join("target-skills");
        fs::create_dir_all(&target_dir).expect("create target dir");

        let config = AppConfig {
            version: 1,
            settings: Settings::default(),
            targets: vec![Target {
                id: "target-1".to_string(),
                name: "Target One".to_string(),
                skills_dir: target_dir.clone(),
                created_at: "1".to_string(),
                updated_at: "1".to_string(),
            }],
            installations: vec![
                Installation {
                    id: "install-1".to_string(),
                    skill_dir_name: "valid-skill".to_string(),
                    skill_name: "valid-skill".to_string(),
                    source_path: valid_skill.path.clone(),
                    target_id: "target-1".to_string(),
                    link_path: target_dir.join("valid-skill"),
                    link_type: fs_adapter::default_link_type(),
                    created_at: "1".to_string(),
                },
                Installation {
                    id: "install-2".to_string(),
                    skill_dir_name: "missing-skill".to_string(),
                    skill_name: "missing-skill".to_string(),
                    source_path: missing_skill.path.clone(),
                    target_id: "target-1".to_string(),
                    link_path: target_dir.join("missing-skill"),
                    link_type: fs_adapter::default_link_type(),
                    created_at: "1".to_string(),
                },
                Installation {
                    id: "install-3".to_string(),
                    skill_dir_name: "mismatch-skill".to_string(),
                    skill_name: "mismatch-skill".to_string(),
                    source_path: mismatch_skill.path.clone(),
                    target_id: "target-1".to_string(),
                    link_path: target_dir.join("mismatch-skill"),
                    link_type: fs_adapter::default_link_type(),
                    created_at: "1".to_string(),
                },
            ],
        };

        fs_adapter::create_dir_link(
            &valid_skill.path,
            &target_dir.join("valid-skill"),
            fs_adapter::default_link_type(),
        )
        .expect("create valid link");

        let other_source = temp.path().join("other-source");
        fs::create_dir_all(&other_source).expect("create other source");
        fs_adapter::create_dir_link(
            &other_source,
            &target_dir.join("mismatch-skill"),
            fs_adapter::default_link_type(),
        )
        .expect("create mismatch link");

        let conflict_dir = target_dir.join("conflict-skill");
        fs::create_dir_all(&conflict_dir).expect("create conflict dir");
        fs::write(conflict_dir.join("existing.txt"), "existing").expect("write conflict file");

        let skills = vec![
            valid_skill.clone(),
            invalid_skill.clone(),
            uninstalled_skill.clone(),
            conflict_skill.clone(),
            missing_skill.clone(),
            mismatch_skill.clone(),
        ];

        let states =
            compute_target_skill_states(&config, "target-1", &skills).expect("compute states");

        assert_eq!(states.len(), 6);

        let valid_state = states
            .iter()
            .find(|s| s.skill.dir_name == "valid-skill")
            .unwrap();
        assert_eq!(valid_state.state, SkillInstallState::Installed);

        let invalid_state = states
            .iter()
            .find(|s| s.skill.dir_name == "invalid-skill")
            .unwrap();
        assert_eq!(invalid_state.state, SkillInstallState::InvalidSkill);

        let uninstalled_state = states
            .iter()
            .find(|s| s.skill.dir_name == "uninstalled-skill")
            .unwrap();
        assert_eq!(uninstalled_state.state, SkillInstallState::NotInstalled);

        let conflict_state = states
            .iter()
            .find(|s| s.skill.dir_name == "conflict-skill")
            .unwrap();
        assert_eq!(conflict_state.state, SkillInstallState::Conflict);

        let missing_state = states
            .iter()
            .find(|s| s.skill.dir_name == "missing-skill")
            .unwrap();
        assert_eq!(missing_state.state, SkillInstallState::Missing);

        let mismatch_state = states
            .iter()
            .find(|s| s.skill.dir_name == "mismatch-skill")
            .unwrap();
        assert_eq!(mismatch_state.state, SkillInstallState::Mismatch);
    }

    #[test]
    fn compute_states_returns_source_missing_when_source_skill_deleted() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let skill = create_valid_skill(&main_dir, "deleted-skill");
        let target_dir = temp.path().join("target-skills");
        fs::create_dir_all(&target_dir).expect("create target dir");

        let mut config = AppConfig {
            version: 1,
            settings: Settings::default(),
            targets: vec![Target {
                id: "target-1".to_string(),
                name: "Target One".to_string(),
                skills_dir: target_dir.clone(),
                created_at: "1".to_string(),
                updated_at: "1".to_string(),
            }],
            installations: Vec::new(),
        };

        // Install the skill first
        install_skill(&mut config, "target-1", "deleted-skill", &[skill.clone()])
            .expect("install skill");

        // Now delete the source skill directory from disk
        fs::remove_dir_all(&skill.path).expect("delete source skill directory");
        assert!(!fs_adapter::path_exists(&skill.path));

        // The skills list is now empty because the source was deleted
        let skills: Vec<SkillView> = vec![];

        let states =
            compute_target_skill_states(&config, "target-1", &skills).expect("compute states");

        assert_eq!(states.len(), 1);
        let source_missing_state = states
            .iter()
            .find(|s| s.skill.dir_name == "deleted-skill")
            .unwrap();
        assert_eq!(source_missing_state.state, SkillInstallState::SourceMissing);
        assert_eq!(source_missing_state.skill.path, skill.path);
    }
}
