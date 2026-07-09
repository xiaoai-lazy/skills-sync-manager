use crate::agent_presets::normalize_platform_path;
use crate::models::{AppConfig, AppError, Project};
use crate::time_util::{current_timestamp, timestamp_nanos};
use std::fs;
use std::path::{Path, PathBuf};

pub fn add_project(
    config: &mut AppConfig,
    name: String,
    root_path: PathBuf,
) -> Result<Project, AppError> {
    let name = validate_project_name(name)?;

    if config
        .projects
        .iter()
        .any(|project| project.name.eq_ignore_ascii_case(&name))
    {
        return Err(AppError::DuplicateProjectName { name });
    }

    validate_project_root(&root_path)?;

    let root_path = normalize_platform_path(root_path);

    let now = current_timestamp();
    let project = Project {
        id: generate_project_id(config),
        name,
        root_path,
        created_at: now.clone(),
        updated_at: now,
    };

    config.projects.push(project.clone());
    Ok(project)
}

pub fn update_project(
    config: &mut AppConfig,
    project_id: &str,
    name: String,
) -> Result<Project, AppError> {
    let name = validate_project_name(name)?;

    let duplicate = config.projects.iter().any(|project| {
        project.id != project_id && project.name.eq_ignore_ascii_case(&name)
    });
    if duplicate {
        return Err(AppError::DuplicateProjectName { name });
    }

    let project = config
        .projects
        .iter_mut()
        .find(|project| project.id == project_id)
        .ok_or_else(|| AppError::ProjectNotFound {
            project_id: project_id.to_string(),
        })?;

    project.name = name;
    project.updated_at = current_timestamp();

    Ok(project.clone())
}

pub fn delete_project(config: &mut AppConfig, project_id: &str) -> Result<(), AppError> {
    if !config.projects.iter().any(|project| project.id == project_id) {
        return Err(AppError::ProjectNotFound {
            project_id: project_id.to_string(),
        });
    }

    let child_target_ids: Vec<String> = config
        .targets
        .iter()
        .filter(|target| target.project_id.as_deref() == Some(project_id))
        .map(|target| target.id.clone())
        .collect();

    let installation_count = config
        .installations
        .iter()
        .filter(|installation| child_target_ids.contains(&installation.target_id))
        .count();

    if installation_count > 0 {
        return Err(AppError::ProjectHasTargetsWithInstallations {
            project_id: project_id.to_string(),
            installation_count,
        });
    }

    config
        .targets
        .retain(|target| target.project_id.as_deref() != Some(project_id));
    config.projects.retain(|project| project.id != project_id);

    Ok(())
}

pub(crate) fn find_project<'a>(
    config: &'a AppConfig,
    project_id: &str,
) -> Result<&'a Project, AppError> {
    config
        .projects
        .iter()
        .find(|project| project.id == project_id)
        .ok_or_else(|| AppError::ProjectNotFound {
            project_id: project_id.to_string(),
        })
}

fn validate_project_name(name: String) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        Err(AppError::InvalidInput {
            input: name,
            message: "Project name must not be blank".to_string(),
        })
    } else {
        Ok(trimmed.to_string())
    }
}

fn validate_project_root(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::InvalidProjectRoot {
            path: path.to_path_buf(),
            message: "Project root does not exist".to_string(),
        });
    }

    if !path.is_dir() {
        return Err(AppError::InvalidProjectRoot {
            path: path.to_path_buf(),
            message: "Project root is not a directory".to_string(),
        });
    }

    let probe_path = path.join(format!(
        ".skills-sync-manager-write-probe-{}-{}",
        std::process::id(),
        timestamp_nanos()
    ));

    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
        .map_err(|err| AppError::InvalidProjectRoot {
            path: path.to_path_buf(),
            message: format!("Project root is not writable: {}", err),
        })?;

    fs::remove_file(&probe_path).map_err(|err| AppError::InvalidProjectRoot {
        path: path.to_path_buf(),
        message: format!("Failed to remove project root write probe: {}", err),
    })?;

    Ok(())
}

fn generate_project_id(config: &AppConfig) -> String {
    loop {
        let id = format!("project-{}", timestamp_nanos());
        if config.projects.iter().all(|project| project.id != id) {
            return id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Installation, LinkType, Target, TargetKind, TargetScope};
    use std::fs;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn add_project_populates_id_and_timestamps() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();

        let project = add_project(
            &mut config,
            "  My App  ".to_string(),
            temp.path().to_path_buf(),
        )
        .expect("add project");

        assert!(project.id.starts_with("project-"));
        assert_eq!(project.name, "My App");
        assert_eq!(project.root_path, temp.path());
        assert!(!project.created_at.is_empty());
        assert_eq!(project.created_at, project.updated_at);
        assert_eq!(config.projects, vec![project]);
    }

    #[test]
    fn add_project_rejects_duplicate_name() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();
        add_project(
            &mut config,
            "My App".to_string(),
            temp.path().to_path_buf(),
        )
        .expect("first project");

        let error = add_project(
            &mut config,
            "my app".to_string(),
            temp.path().join("other"),
        )
        .expect_err("duplicate name should fail");

        assert!(matches!(
            error,
            AppError::DuplicateProjectName { name } if name == "my app"
        ));
        assert_eq!(config.projects.len(), 1);
    }

    #[test]
    fn update_project_changes_name_only() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();
        let project = add_project(
            &mut config,
            "Old Name".to_string(),
            temp.path().to_path_buf(),
        )
        .expect("add project");
        thread::sleep(Duration::from_nanos(1));

        let updated = update_project(&mut config, &project.id, "  New Name  ".to_string())
            .expect("update project");

        assert_eq!(updated.id, project.id);
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.root_path, temp.path());
        assert_eq!(updated.created_at, project.created_at);
        assert_ne!(updated.updated_at, project.created_at);
    }

    #[test]
    fn delete_project_refuses_when_child_targets_have_installations() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();
        let project = add_project(
            &mut config,
            "My App".to_string(),
            temp.path().to_path_buf(),
        )
        .expect("add project");
        config.targets.push(Target {
            id: "target-1".to_string(),
            name: "Cursor".to_string(),
            scope: TargetScope::Project,
            kind: TargetKind::Agent,
            agent_id: Some("cursor".to_string()),
            project_id: Some(project.id.clone()),
            custom_path: None,
            skills_dir: temp.path().join(".cursor").join("skills"),
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        });
        fs::create_dir_all(&config.targets[0].skills_dir).expect("create skills dir");
        config.installations.push(Installation {
            id: "install-1".to_string(),
            skill_dir_name: "example-skill".to_string(),
            skill_name: "Example Skill".to_string(),
            source_path: temp.path().join("main").join("example-skill"),
            target_id: "target-1".to_string(),
            link_path: config.targets[0].skills_dir.join("example-skill"),
            link_type: LinkType::Symlink,
            created_at: "1".to_string(),
            ..Default::default()
        });

        let error = delete_project(&mut config, &project.id).expect_err("should refuse delete");

        assert!(matches!(
            error,
            AppError::ProjectHasTargetsWithInstallations {
                project_id,
                installation_count: 1
            } if project_id == project.id
        ));
        assert_eq!(config.projects.len(), 1);
        assert_eq!(config.targets.len(), 1);
    }

    #[test]
    fn delete_project_removes_project_and_child_targets_without_installations() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();
        let project = add_project(
            &mut config,
            "My App".to_string(),
            temp.path().to_path_buf(),
        )
        .expect("add project");
        config.targets.push(Target {
            id: "target-1".to_string(),
            name: "Cursor".to_string(),
            scope: TargetScope::Project,
            kind: TargetKind::Agent,
            agent_id: Some("cursor".to_string()),
            project_id: Some(project.id.clone()),
            custom_path: None,
            skills_dir: temp.path().join(".cursor").join("skills"),
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        });

        delete_project(&mut config, &project.id).expect("delete project");

        assert!(config.projects.is_empty());
        assert!(config.targets.is_empty());
    }
}
