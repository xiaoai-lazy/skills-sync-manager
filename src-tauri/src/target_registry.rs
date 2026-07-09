use crate::agent_presets::{normalize_path_for_compare, normalize_platform_path, resolve_skills_dir, AgentPreset};
use crate::models::{
    AppConfig, AppError, Target, TargetKind, TargetScope,
};
use crate::project_registry::find_project;
use crate::time_util::{current_timestamp, timestamp_nanos};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddTargetRequest {
    pub name: String,
    pub skills_dir: PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddAgentTargetRequest {
    pub scope: TargetScope,
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddCustomTargetRequest {
    pub scope: TargetScope,
    pub name: String,
    pub skills_dir: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTargetRequest {
    pub name: String,
}

pub fn add_target(config: &mut AppConfig, request: AddTargetRequest) -> Result<Target, AppError> {
    add_custom_target(
        config,
        AddCustomTargetRequest {
            scope: TargetScope::Global,
            name: request.name,
            skills_dir: request.skills_dir,
            project_id: None,
        },
    )
}

pub fn add_agent_target(
    config: &mut AppConfig,
    presets: &[AgentPreset],
    request: AddAgentTargetRequest,
) -> Result<Target, AppError> {
    let preset = presets
        .iter()
        .find(|preset| preset.id == request.agent_id)
        .ok_or_else(|| AppError::InvalidInput {
            input: request.agent_id.clone(),
            message: "unknown agent preset".to_string(),
        })?;

    let project_root = resolve_scope_project(config, &request.scope, request.project_id.as_deref())?;
    let skills_dir = resolve_skills_dir(preset, request.scope.clone(), project_root)?;
    ensure_agent_target_dir(&skills_dir)?;

    check_duplicate_target(
        config,
        &request.scope,
        request.project_id.as_deref(),
        &preset.display_name,
        &skills_dir,
        Some(&request.agent_id),
    )?;

    let now = current_timestamp();
    let target = Target {
        id: generate_target_id(config),
        name: preset.display_name.clone(),
        scope: request.scope,
        kind: TargetKind::Agent,
        agent_id: Some(preset.id.clone()),
        project_id: request.project_id,
        custom_path: None,
        skills_dir,
        created_at: now.clone(),
        updated_at: now,
    };

    config.targets.push(target.clone());
    Ok(target)
}

pub fn add_custom_target(
    config: &mut AppConfig,
    request: AddCustomTargetRequest,
) -> Result<Target, AppError> {
    let name = validate_target_name(request.name)?;
    let skills_dir = normalize_platform_path(request.skills_dir);
    validate_target_dir(&skills_dir)?;

    let project_root = resolve_scope_project(config, &request.scope, request.project_id.as_deref())?;
    if let Some(root) = project_root {
        validate_path_under_project_root(&skills_dir, root)?;
    }

    check_duplicate_target(
        config,
        &request.scope,
        request.project_id.as_deref(),
        &name,
        &skills_dir,
        None,
    )?;

    let now = current_timestamp();
    let target = Target {
        id: generate_target_id(config),
        name,
        scope: request.scope,
        kind: TargetKind::Custom,
        agent_id: None,
        project_id: request.project_id,
        custom_path: Some(skills_dir.clone()),
        skills_dir,
        created_at: now.clone(),
        updated_at: now,
    };

    config.targets.push(target.clone());
    Ok(target)
}

pub fn update_target(
    config: &mut AppConfig,
    target_id: &str,
    request: UpdateTargetRequest,
) -> Result<Target, AppError> {
    let name = validate_target_name(request.name)?;

    let target = config
        .targets
        .iter_mut()
        .find(|target| target.id == target_id)
        .ok_or_else(|| AppError::TargetNotFound {
            target_id: target_id.to_string(),
        })?;

    if target.kind == TargetKind::Agent {
        return Err(AppError::TargetNotEditable {
            target_id: target_id.to_string(),
        });
    }

    target.name = name;
    target.updated_at = current_timestamp();

    Ok(target.clone())
}

pub fn delete_target_config(config: &mut AppConfig, target_id: &str) -> Result<(), AppError> {
    let installation_count = config
        .installations
        .iter()
        .filter(|installation| installation.target_id == target_id)
        .count();

    if installation_count > 0 {
        return Err(AppError::TargetHasInstallations {
            target_id: target_id.to_string(),
            installation_count,
        });
    }

    let original_len = config.targets.len();
    config.targets.retain(|target| target.id != target_id);

    if config.targets.len() == original_len {
        return Err(AppError::TargetNotFound {
            target_id: target_id.to_string(),
        });
    }

    Ok(())
}

fn ensure_agent_target_dir(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|err| AppError::InvalidTargetDir {
            path: path.to_path_buf(),
            message: format!("Failed to create target skills directory: {}", err),
        })?;
    }

    validate_target_dir(path)
}

pub fn validate_target_dir(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::InvalidTargetDir {
            path: path.to_path_buf(),
            message: "Target skills directory does not exist".to_string(),
        });
    }

    if !path.is_dir() {
        return Err(AppError::InvalidTargetDir {
            path: path.to_path_buf(),
            message: "Target skills directory is not a directory".to_string(),
        });
    }

    let probe_path = path.join(format!(
        ".skills-sync-manager-write-probe-{}-{}",
        std::process::id(),
        timestamp_nanos()
    ));

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
        .map_err(|err| AppError::InvalidTargetDir {
            path: path.to_path_buf(),
            message: format!("Target skills directory is not writable: {}", err),
        })?;

    fs::remove_file(&probe_path).map_err(|err| AppError::InvalidTargetDir {
        path: path.to_path_buf(),
        message: format!("Failed to remove target directory write probe: {}", err),
    })?;

    Ok(())
}

fn resolve_scope_project<'a>(
    config: &'a AppConfig,
    scope: &TargetScope,
    project_id: Option<&str>,
) -> Result<Option<&'a Path>, AppError> {
    match scope {
        TargetScope::Global => {
            if project_id.is_some() {
                return Err(AppError::InvalidInput {
                    input: project_id.unwrap_or("").to_string(),
                    message: "project_id must not be set for global scope".to_string(),
                });
            }
            Ok(None)
        }
        TargetScope::Project => {
            let project_id = project_id.ok_or_else(|| AppError::InvalidInput {
                input: String::new(),
                message: "project_id is required for project scope".to_string(),
            })?;
            let project = find_project(config, project_id)?;
            Ok(Some(project.root_path.as_path()))
        }
    }
}

fn validate_path_under_project_root(skills_dir: &Path, project_root: &Path) -> Result<(), AppError> {
    let normalized_skills = normalize_path_for_compare(skills_dir);
    let normalized_root = normalize_path_for_compare(project_root);
    let root_prefix = if normalized_root.ends_with('/') {
        normalized_root.clone()
    } else {
        format!("{}/", normalized_root)
    };

    if normalized_skills != normalized_root && !normalized_skills.starts_with(&root_prefix) {
        return Err(AppError::PathOutsideProjectRoot {
            path: skills_dir.to_path_buf(),
            project_root: project_root.to_path_buf(),
        });
    }

    Ok(())
}

fn check_duplicate_target(
    config: &AppConfig,
    scope: &TargetScope,
    project_id: Option<&str>,
    name: &str,
    skills_dir: &Path,
    agent_id: Option<&str>,
) -> Result<(), AppError> {
    let normalized_skills_dir = normalize_path_for_compare(skills_dir);

    for target in targets_in_scope(config, scope, project_id) {
        if target.name.eq_ignore_ascii_case(name) {
            return Err(AppError::DuplicateTarget {
                message: format!("target name already exists: {}", name),
            });
        }

        if normalize_path_for_compare(&target.skills_dir) == normalized_skills_dir {
            return Err(AppError::DuplicateTarget {
                message: format!(
                    "target skills directory already exists: {}",
                    skills_dir.display()
                ),
            });
        }

        if let Some(agent_id) = agent_id {
            if target.agent_id.as_deref() == Some(agent_id) {
                return Err(AppError::DuplicateTarget {
                    message: format!("agent preset already added: {}", agent_id),
                });
            }
        }
    }

    Ok(())
}

fn targets_in_scope<'a>(
    config: &'a AppConfig,
    scope: &TargetScope,
    project_id: Option<&str>,
) -> Vec<&'a Target> {
    config
        .targets
        .iter()
        .filter(|target| match scope {
            TargetScope::Global => target.scope == TargetScope::Global,
            TargetScope::Project => {
                target.scope == TargetScope::Project && target.project_id.as_deref() == project_id
            }
        })
        .collect()
}

fn validate_target_name(name: String) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        Err(AppError::InvalidTargetName)
    } else {
        Ok(trimmed.to_string())
    }
}

fn generate_target_id(config: &AppConfig) -> String {
    loop {
        let id = format!("target-{}", timestamp_nanos());
        if config.targets.iter().all(|target| target.id != id) {
            return id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_presets::builtin_presets;
    use crate::models::{Installation, LinkType};
    use crate::project_registry::add_project;
    use std::fs;
    use std::thread;
    use std::time::Duration;

    fn add_request(name: &str, skills_dir: PathBuf) -> AddTargetRequest {
        AddTargetRequest {
            name: name.to_string(),
            skills_dir,
        }
    }

    fn update_request(name: &str) -> UpdateTargetRequest {
        UpdateTargetRequest {
            name: name.to_string(),
        }
    }

    fn existing_target(id: &str, skills_dir: PathBuf) -> Target {
        Target::global_custom(id, "Old Target", skills_dir, "1", "1")
    }

    fn agent_target(id: &str, agent_id: &str, skills_dir: PathBuf) -> Target {
        Target {
            id: id.to_string(),
            name: "Cursor".to_string(),
            scope: TargetScope::Global,
            kind: TargetKind::Agent,
            agent_id: Some(agent_id.to_string()),
            project_id: None,
            custom_path: None,
            skills_dir,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        }
    }

    #[test]
    fn add_target_populates_id_and_timestamps() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();

        let target = add_target(
            &mut config,
            add_request("  Claude Code  ", temp.path().to_path_buf()),
        )
        .expect("add target");

        assert!(target.id.starts_with("target-"));
        assert_eq!(target.name, "Claude Code");
        assert_eq!(target.skills_dir, temp.path());
        assert_eq!(target.scope, TargetScope::Global);
        assert_eq!(target.kind, TargetKind::Custom);
        assert!(!target.created_at.is_empty());
        assert_eq!(target.created_at, target.updated_at);
        assert_eq!(config.targets, vec![target]);
    }

    #[test]
    fn add_target_rejects_blank_name() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();

        let error = add_target(
            &mut config,
            add_request("  \t  ", temp.path().to_path_buf()),
        )
        .expect_err("blank name should fail");

        assert!(matches!(error, AppError::InvalidTargetName));
        assert!(config.targets.is_empty());
    }

    #[test]
    fn add_agent_target_global_dedupes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();
        let presets = builtin_presets();
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .expect("home dir");
        let cursor_dir = home.join(".cursor").join("skills");
        fs::create_dir_all(&cursor_dir).expect("create cursor dir");

        add_agent_target(
            &mut config,
            &presets,
            AddAgentTargetRequest {
                scope: TargetScope::Global,
                agent_id: "cursor".to_string(),
                project_id: None,
            },
        )
        .expect("first agent target");

        let error = add_agent_target(
            &mut config,
            &presets,
            AddAgentTargetRequest {
                scope: TargetScope::Global,
                agent_id: "cursor".to_string(),
                project_id: None,
            },
        )
        .expect_err("duplicate agent should fail");

        assert!(matches!(error, AppError::DuplicateTarget { .. }));
        assert_eq!(config.targets.len(), 1);

        let _ = temp;
    }

    #[test]
    fn add_agent_target_project_requires_project_relative_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();
        let project = add_project(
            &mut config,
            "My App".to_string(),
            temp.path().to_path_buf(),
        )
        .expect("add project");
        let presets = vec![AgentPreset {
            id: "opencode".to_string(),
            display_name: "OpenCode".to_string(),
            global_path: "~/.opencode/skills".to_string(),
            project_relative_path: None,
            icon: None,
        }];

        let error = add_agent_target(
            &mut config,
            &presets,
            AddAgentTargetRequest {
                scope: TargetScope::Project,
                agent_id: "opencode".to_string(),
                project_id: Some(project.id),
            },
        )
        .expect_err("missing project relative path should fail");

        assert!(matches!(error, AppError::InvalidInput { .. }));
        assert!(config.targets.is_empty());
    }

    #[test]
    fn add_agent_target_creates_missing_project_skills_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();
        let project = add_project(
            &mut config,
            "My App".to_string(),
            temp.path().to_path_buf(),
        )
        .expect("add project");
        let presets = builtin_presets();
        let skills_dir = temp.path().join(".codex").join("skills");
        assert!(!skills_dir.exists());

        let target = add_agent_target(
            &mut config,
            &presets,
            AddAgentTargetRequest {
                scope: TargetScope::Project,
                agent_id: "codex".to_string(),
                project_id: Some(project.id),
            },
        )
        .expect("add agent target");

        assert!(skills_dir.is_dir());
        assert_eq!(target.skills_dir, skills_dir);
    }

    #[test]
    fn add_custom_target_project_path_must_be_under_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        let mut config = AppConfig::default();
        let project = add_project(
            &mut config,
            "My App".to_string(),
            temp.path().to_path_buf(),
        )
        .expect("add project");
        fs::create_dir_all(outside.path()).expect("create outside dir");

        let error = add_custom_target(
            &mut config,
            AddCustomTargetRequest {
                scope: TargetScope::Project,
                name: "Tools".to_string(),
                skills_dir: outside.path().to_path_buf(),
                project_id: Some(project.id),
            },
        )
        .expect_err("outside path should fail");

        assert!(matches!(
            error,
            AppError::PathOutsideProjectRoot { .. }
        ));
        assert!(config.targets.is_empty());
    }

    #[test]
    fn add_custom_target_project_accepts_path_under_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let skills_dir = temp.path().join(".cursor").join("skills");
        fs::create_dir_all(&skills_dir).expect("create skills dir");
        let mut config = AppConfig::default();
        let project = add_project(
            &mut config,
            "My App".to_string(),
            temp.path().to_path_buf(),
        )
        .expect("add project");

        let target = add_custom_target(
            &mut config,
            AddCustomTargetRequest {
                scope: TargetScope::Project,
                name: "Tools".to_string(),
                skills_dir: skills_dir.clone(),
                project_id: Some(project.id),
            },
        )
        .expect("add project custom target");

        assert_eq!(target.scope, TargetScope::Project);
        assert_eq!(target.skills_dir, skills_dir);
    }

    #[test]
    fn update_target_agent_fails() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();
        config
            .targets
            .push(agent_target("target-1", "cursor", temp.path().to_path_buf()));

        let error = update_target(
            &mut config,
            "target-1",
            update_request("New Name"),
        )
        .expect_err("agent target should not be editable");

        assert!(matches!(
            error,
            AppError::TargetNotEditable { target_id } if target_id == "target-1"
        ));
    }

    #[test]
    fn update_target_custom_name_only_no_path_change() {
        let first = tempfile::tempdir().expect("first tempdir");
        let mut config = AppConfig::default();
        config
            .targets
            .push(existing_target("target-1", first.path().to_path_buf()));
        thread::sleep(Duration::from_nanos(1));

        let target = update_target(
            &mut config,
            "target-1",
            update_request("  Updated Target  "),
        )
        .expect("update target");

        assert_eq!(target.id, "target-1");
        assert_eq!(target.name, "Updated Target");
        assert_eq!(target.skills_dir, first.path());
        assert_eq!(target.created_at, "1");
        assert_ne!(target.updated_at, "1");
        assert_eq!(config.targets[0], target);
    }

    #[test]
    fn update_target_rejects_missing_target() {
        let mut config = AppConfig::default();

        let error = update_target(
            &mut config,
            "missing-target",
            update_request("Target"),
        )
        .expect_err("missing target should fail");

        assert!(matches!(
            error,
            AppError::TargetNotFound { target_id } if target_id == "missing-target"
        ));
    }

    #[test]
    fn delete_target_config_removes_record_but_not_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let target_dir = temp.path().join("target-skills");
        fs::create_dir_all(&target_dir).expect("create target dir");
        let mut config = AppConfig::default();
        config
            .targets
            .push(existing_target("target-1", target_dir.clone()));

        delete_target_config(&mut config, "target-1").expect("delete target config");

        assert!(config.targets.is_empty());
        assert!(target_dir.is_dir());
    }

    #[test]
    fn delete_target_config_refuses_when_installation_records_exist() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();
        config
            .targets
            .push(existing_target("target-1", temp.path().to_path_buf()));
        config.installations.push(Installation {
            id: "install-1".to_string(),
            skill_dir_name: "example-skill".to_string(),
            skill_name: "Example Skill".to_string(),
            source_path: temp.path().join("main").join("example-skill"),
            target_id: "target-1".to_string(),
            link_path: temp.path().join("target").join("example-skill"),
            link_type: LinkType::Symlink,
            created_at: "1".to_string(),
            ..Default::default()
        });

        let error = delete_target_config(&mut config, "target-1")
            .expect_err("target with installs should fail");

        assert!(matches!(
            error,
            AppError::TargetHasInstallations {
                target_id,
                installation_count: 1
            } if target_id == "target-1"
        ));
        assert_eq!(config.targets.len(), 1);
    }

    #[test]
    fn validate_target_dir_fails_for_missing_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let missing = temp.path().join("missing");

        let error = validate_target_dir(&missing).expect_err("missing path should fail");

        assert!(matches!(error, AppError::InvalidTargetDir { .. }));
    }

    #[test]
    fn validate_target_dir_fails_for_regular_file_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file_path = temp.path().join("not-a-directory");
        fs::write(&file_path, "not a directory").expect("write file");

        let error = validate_target_dir(&file_path).expect_err("file path should fail");

        assert!(matches!(error, AppError::InvalidTargetDir { .. }));
    }

    #[test]
    fn validate_target_dir_accepts_writable_directory() {
        let temp = tempfile::tempdir().expect("tempdir");

        validate_target_dir(temp.path()).expect("writable directory should pass");

        let probe_entries = fs::read_dir(temp.path())
            .expect("read target dir")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".skills-sync-manager-write-probe-")
            })
            .count();
        assert_eq!(probe_entries, 0);
    }
}
