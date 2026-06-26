use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub settings: Settings,
    pub targets: Vec<Target>,
    pub installations: Vec<Installation>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            settings: Settings::default(),
            targets: Vec::new(),
            installations: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub main_skills_dir: Option<PathBuf>,
    pub link_strategy: LinkStrategy,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            main_skills_dir: None,
            link_strategy: LinkStrategy::Auto,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinkStrategy {
    Auto,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub id: String,
    pub name: String,
    pub skills_dir: PathBuf,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub id: String,
    pub skill_dir_name: String,
    pub skill_name: String,
    pub source_path: PathBuf,
    pub target_id: String,
    pub link_path: PathBuf,
    pub link_type: LinkType,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinkType {
    Junction,
    Symlink,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillView {
    pub dir_name: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub path: PathBuf,
    pub valid: bool,
    pub validation_errors: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SkillInstallState {
    NotInstalled,
    Installed,
    Conflict,
    Missing,
    Mismatch,
    SourceMissing,
    InvalidSkill,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillWithTargetState {
    pub skill: SkillView,
    pub state: SkillInstallState,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMainSkillResult {
    pub deleted_skill_dir_name: String,
    pub removed_link_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub config: AppConfig,
    pub skills: Vec<SkillView>,
    pub selected_target_id: Option<String>,
    pub selected_target_skills: Vec<SkillWithTargetState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    ConfigRead {
        path: PathBuf,
        message: String,
    },
    ConfigParse {
        path: PathBuf,
        message: String,
    },
    ConfigWrite {
        path: PathBuf,
        message: String,
    },
    InvalidMainSkillsDir {
        path: PathBuf,
        message: String,
    },
    InvalidSkill {
        skill_dir_name: String,
        message: String,
    },
    Conflict {
        path: PathBuf,
        message: String,
    },
    TargetNotFound {
        target_id: String,
    },
    InvalidTargetName,
    InvalidTargetDir {
        path: PathBuf,
        message: String,
    },
    TargetHasInstallations {
        target_id: String,
        installation_count: usize,
    },
    Io {
        path: Option<PathBuf>,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn to_dto(&self) -> AppErrorDto {
        match self {
            AppError::ConfigRead { path, message } => AppErrorDto {
                code: "configRead".to_string(),
                message: format!("Failed to read config at {}: {}", path.display(), message),
            },
            AppError::ConfigParse { path, message } => AppErrorDto {
                code: "configParse".to_string(),
                message: format!("Failed to parse config at {}: {}", path.display(), message),
            },
            AppError::ConfigWrite { path, message } => AppErrorDto {
                code: "configWrite".to_string(),
                message: format!("Failed to write config at {}: {}", path.display(), message),
            },
            AppError::InvalidMainSkillsDir { path, message } => AppErrorDto {
                code: "invalidMainSkillsDir".to_string(),
                message: format!("Invalid main skills directory at {}: {}", path.display(), message),
            },
            AppError::InvalidSkill {
                skill_dir_name,
                message,
            } => AppErrorDto {
                code: "invalidSkill".to_string(),
                message: format!("Invalid skill '{}': {}", skill_dir_name, message),
            },
            AppError::Conflict { path, message } => AppErrorDto {
                code: "conflict".to_string(),
                message: format!(
                    "目标路径已存在，无法安装：{} ({})",
                    path.display(),
                    message
                ),
            },
            AppError::TargetNotFound { target_id } => AppErrorDto {
                code: "targetNotFound".to_string(),
                message: format!("Target not found: {}", target_id),
            },
            AppError::InvalidTargetName => AppErrorDto {
                code: "invalidTargetName".to_string(),
                message: "Target name must not be blank".to_string(),
            },
            AppError::InvalidTargetDir { path, message } => AppErrorDto {
                code: "invalidTargetDir".to_string(),
                message: format!("Invalid target directory at {}: {}", path.display(), message),
            },
            AppError::TargetHasInstallations {
                target_id,
                installation_count,
            } => AppErrorDto {
                code: "targetHasInstallations".to_string(),
                message: format!(
                    "Target {} still has {} installation record(s)",
                    target_id, installation_count
                ),
            },
            AppError::Io { path, message } => AppErrorDto {
                code: "io".to_string(),
                message: match path {
                    Some(path) => format!("Filesystem error at {}: {}", path.display(), message),
                    None => format!("Filesystem error: {}", message),
                },
            },
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::ConfigRead { path, message } => {
                write!(
                    formatter,
                    "failed to read config at {}: {}",
                    path.display(),
                    message
                )
            }
            AppError::ConfigParse { path, message } => {
                write!(
                    formatter,
                    "failed to parse config at {}: {}",
                    path.display(),
                    message
                )
            }
            AppError::ConfigWrite { path, message } => {
                write!(
                    formatter,
                    "failed to write config at {}: {}",
                    path.display(),
                    message
                )
            }
            AppError::InvalidMainSkillsDir { path, message } => {
                write!(
                    formatter,
                    "invalid main skills directory at {}: {}",
                    path.display(),
                    message
                )
            }
            AppError::InvalidSkill {
                skill_dir_name,
                message,
            } => write!(formatter, "invalid skill '{}': {}", skill_dir_name, message),
            AppError::Conflict { path, message } => {
                write!(formatter, "conflict at {}: {}", path.display(), message)
            }
            AppError::TargetNotFound { target_id } => {
                write!(formatter, "target not found: {}", target_id)
            }
            AppError::InvalidTargetName => write!(formatter, "target name must not be blank"),
            AppError::InvalidTargetDir { path, message } => {
                write!(
                    formatter,
                    "invalid target directory at {}: {}",
                    path.display(),
                    message
                )
            }
            AppError::TargetHasInstallations {
                target_id,
                installation_count,
            } => write!(
                formatter,
                "target {} still has {} installation record(s)",
                target_id, installation_count
            ),
            AppError::Io { path, message } => {
                if let Some(path) = path {
                    write!(
                        formatter,
                        "filesystem error at {}: {}",
                        path.display(),
                        message
                    )
                } else {
                    write!(formatter, "filesystem error: {}", message)
                }
            }
        }
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn installation_serializes_with_camel_case_fields() {
        let installation = Installation {
            id: "install-1".to_string(),
            skill_dir_name: "example-skill".to_string(),
            skill_name: "Example Skill".to_string(),
            source_path: PathBuf::from("C:/skills/example-skill"),
            target_id: "target-1".to_string(),
            link_path: PathBuf::from("C:/target/skills/example-skill"),
            link_type: LinkType::Junction,
            created_at: "2026-06-23T00:00:00Z".to_string(),
        };

        let value = serde_json::to_value(installation).expect("installation serializes");
        let object = value
            .as_object()
            .expect("installation serializes to object");

        assert!(object.contains_key("skillDirName"));
        assert!(object.contains_key("skillName"));
        assert!(object.contains_key("sourcePath"));
        assert!(object.contains_key("targetId"));
        assert!(object.contains_key("linkPath"));
        assert!(object.contains_key("linkType"));
        assert!(object.contains_key("createdAt"));
        assert!(!object.contains_key("skill_dir_name"));
        assert!(!object.contains_key("source_path"));
    }

    #[test]
    fn enums_serialize_to_type_script_union_values() {
        assert_eq!(
            serde_json::to_value(LinkStrategy::Auto).unwrap(),
            json!("auto")
        );
        assert_eq!(
            serde_json::to_value(LinkType::Junction).unwrap(),
            json!("junction")
        );
        assert_eq!(
            serde_json::to_value(LinkType::Symlink).unwrap(),
            json!("symlink")
        );
        assert_eq!(
            serde_json::to_value(SkillInstallState::NotInstalled).unwrap(),
            json!("notInstalled")
        );
        assert_eq!(
            serde_json::to_value(SkillInstallState::Installed).unwrap(),
            json!("installed")
        );
        assert_eq!(
            serde_json::to_value(SkillInstallState::Conflict).unwrap(),
            json!("conflict")
        );
        assert_eq!(
            serde_json::to_value(SkillInstallState::Missing).unwrap(),
            json!("missing")
        );
        assert_eq!(
            serde_json::to_value(SkillInstallState::Mismatch).unwrap(),
            json!("mismatch")
        );
        assert_eq!(
            serde_json::to_value(SkillInstallState::SourceMissing).unwrap(),
            json!("sourceMissing")
        );
        assert_eq!(
            serde_json::to_value(SkillInstallState::InvalidSkill).unwrap(),
            json!("invalidSkill")
        );
    }
}
