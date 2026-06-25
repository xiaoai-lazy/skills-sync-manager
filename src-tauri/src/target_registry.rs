use crate::models::{AppConfig, AppError, Target};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddTargetRequest {
    pub name: String,
    pub skills_dir: PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTargetRequest {
    pub name: String,
    pub skills_dir: PathBuf,
}

pub fn add_target(config: &mut AppConfig, request: AddTargetRequest) -> Result<Target, AppError> {
    let name = validate_target_name(request.name)?;
    validate_target_dir(&request.skills_dir)?;

    let now = current_timestamp();
    let target = Target {
        id: generate_target_id(config),
        name,
        skills_dir: request.skills_dir,
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
    validate_target_dir(&request.skills_dir)?;

    let target = config
        .targets
        .iter_mut()
        .find(|target| target.id == target_id)
        .ok_or_else(|| AppError::TargetNotFound {
            target_id: target_id.to_string(),
        })?;

    target.name = name;
    target.skills_dir = request.skills_dir;
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
    use crate::models::{Installation, LinkType};
    use std::fs;
    use std::thread;
    use std::time::Duration;

    fn add_request(name: &str, skills_dir: PathBuf) -> AddTargetRequest {
        AddTargetRequest {
            name: name.to_string(),
            skills_dir,
        }
    }

    fn update_request(name: &str, skills_dir: PathBuf) -> UpdateTargetRequest {
        UpdateTargetRequest {
            name: name.to_string(),
            skills_dir,
        }
    }

    fn existing_target(id: &str, skills_dir: PathBuf) -> Target {
        Target {
            id: id.to_string(),
            name: "Old Target".to_string(),
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
    fn update_target_changes_name_path_and_updated_at() {
        let first = tempfile::tempdir().expect("first tempdir");
        let second = tempfile::tempdir().expect("second tempdir");
        let mut config = AppConfig::default();
        config
            .targets
            .push(existing_target("target-1", first.path().to_path_buf()));
        thread::sleep(Duration::from_nanos(1));

        let target = update_target(
            &mut config,
            "target-1",
            update_request("  Updated Target  ", second.path().to_path_buf()),
        )
        .expect("update target");

        assert_eq!(target.id, "target-1");
        assert_eq!(target.name, "Updated Target");
        assert_eq!(target.skills_dir, second.path());
        assert_eq!(target.created_at, "1");
        assert_ne!(target.updated_at, "1");
        assert_eq!(config.targets[0], target);
    }

    #[test]
    fn update_target_rejects_missing_target() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = AppConfig::default();

        let error = update_target(
            &mut config,
            "missing-target",
            update_request("Target", temp.path().to_path_buf()),
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
