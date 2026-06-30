use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Current on-disk config schema version. Bump when adding breaking fields.
pub const CURRENT_CONFIG_VERSION: u32 = 3;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub settings: Settings,
    pub targets: Vec<Target>,
    pub installations: Vec<Installation>,
    #[serde(default)]
    pub skill_repos: Vec<SkillRepo>,
    #[serde(default)]
    pub skill_records: HashMap<String, SkillRecord>,
    #[serde(default)]
    pub skill_discover_cache: SkillDiscoverCache,
    #[serde(default)]
    pub skill_update_cache: SkillUpdateCache,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_CONFIG_VERSION,
            settings: Settings::default(),
            targets: Vec::new(),
            installations: Vec::new(),
            skill_repos: vec![SkillRepo {
                owner: "obra".to_string(),
                name: "superpowers".to_string(),
                branch: "main".to_string(),
                enabled: true,
            }],
            skill_records: HashMap::new(),
            skill_discover_cache: SkillDiscoverCache::default(),
            skill_update_cache: SkillUpdateCache::default(),
        }
    }
}

/// Upgrade an on-disk config to the current schema. Returns true when persisted state changed.
pub fn migrate_config(config: &mut AppConfig) -> bool {
    if config.version >= CURRENT_CONFIG_VERSION {
        return false;
    }
    config.version = CURRENT_CONFIG_VERSION;
    true
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

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRepo {
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
    pub source: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub directory: String,
    pub content_hash: String,
    pub installed_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillDiscoverCache {
    pub fetched_at: Option<String>,
    pub skills: Vec<DiscoverableSkill>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateCache {
    pub checked_at: Option<String>,
    pub updates: Vec<SkillUpdateInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverableSkill {
    pub key: String,
    pub name: String,
    pub description: String,
    pub directory: String,
    pub install_dir_name: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub source: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRepoChangeResult {
    pub repos: Vec<SkillRepo>,
    pub discover_skills: Vec<DiscoverableSkill>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateInfo {
    pub dir_name: String,
    pub name: String,
    pub current_hash: Option<String>,
    pub remote_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAllSkillsResult {
    pub updated: Vec<String>,
    pub failed: Vec<UpdateAllSkillsFailure>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAllSkillsFailure {
    pub dir_name: String,
    pub error: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillHubLocalState {
    pub skills: Vec<SkillView>,
    pub valid_count: u32,
    pub invalid_count: u32,
    pub pending_update_count: u32,
    pub last_scan_at: String,
    pub skill_records: HashMap<String, SkillRecord>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SmartPastePreview {
    pub name: String,
    pub description: String,
    pub install_dir_name: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub directory: String,
    pub source: String,
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
    DownloadFailed {
        url: String,
        status: Option<u16>,
        message: String,
    },
    DiscoverInProgress,
    DirExists {
        path: PathBuf,
    },
    SkillDirNotFound {
        path: PathBuf,
    },
    UpdatesInProgress,
    UpdateNotPending {
        dir_name: String,
    },
    InvalidInput {
        input: String,
        message: String,
    },
    MissingSkillPath {
        input: String,
    },
    SkillRepoNotFound {
        owner: String,
        name: String,
    },
}

pub const SMART_PASTE_GITHUB_EXAMPLE: &str =
    "https://github.com/obra/superpowers/blob/main/skills/brainstorming/SKILL.md";

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
                message: format!("无法读取配置文件 {}: {}", path.display(), message),
            },
            AppError::ConfigParse { path, message } => AppErrorDto {
                code: "configParse".to_string(),
                message: format!("无法解析配置文件 {}: {}", path.display(), message),
            },
            AppError::ConfigWrite { path, message } => AppErrorDto {
                code: "configWrite".to_string(),
                message: format!("无法写入配置文件 {}: {}", path.display(), message),
            },
            AppError::InvalidMainSkillsDir { path, message } => AppErrorDto {
                code: "invalidMainSkillsDir".to_string(),
                message: format!("主 skill 目录无效 {}: {}", path.display(), message),
            },
            AppError::InvalidSkill {
                skill_dir_name,
                message,
            } => AppErrorDto {
                code: "invalidSkill".to_string(),
                message: format!("Skill 无效 '{}': {}", skill_dir_name, message),
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
                message: format!("找不到目标 {}", target_id),
            },
            AppError::InvalidTargetName => AppErrorDto {
                code: "invalidTargetName".to_string(),
                message: "目标名称不能为空".to_string(),
            },
            AppError::InvalidTargetDir { path, message } => AppErrorDto {
                code: "invalidTargetDir".to_string(),
                message: format!("目标目录无效 {}: {}", path.display(), message),
            },
            AppError::TargetHasInstallations {
                target_id,
                installation_count,
            } => AppErrorDto {
                code: "targetHasInstallations".to_string(),
                message: format!(
                    "目标 {} 仍有 {} 条安装记录",
                    target_id, installation_count
                ),
            },
            AppError::Io { path, message } => AppErrorDto {
                code: "io".to_string(),
                message: match path {
                    Some(path) => format!("文件系统错误 {}: {}", path.display(), message),
                    None => format!("文件系统错误：{}", message),
                },
            },
            AppError::DownloadFailed {
                url,
                status,
                message,
            } => AppErrorDto {
                code: "downloadFailed".to_string(),
                message: match status {
                    Some(code) => format!("下载失败 {} (HTTP {}): {}", url, code, message),
                    None => format!("下载失败 {}: {}", url, message),
                },
            },
            AppError::DiscoverInProgress => AppErrorDto {
                code: "discoverInProgress".to_string(),
                message: "Skill 发现正在进行中，请稍后再试".to_string(),
            },
            AppError::DirExists { path } => AppErrorDto {
                code: "dirExists".to_string(),
                message: format!("目标目录已存在：{}", path.display()),
            },
            AppError::SkillDirNotFound { path } => AppErrorDto {
                code: "skillDirNotFound".to_string(),
                message: format!("未找到有效的 Skill 目录：{}", path.display()),
            },
            AppError::UpdatesInProgress => AppErrorDto {
                code: "updatesInProgress".to_string(),
                message: "Skill 更新检查正在进行中，请稍后再试".to_string(),
            },
            AppError::UpdateNotPending { dir_name } => AppErrorDto {
                code: "notPending".to_string(),
                message: format!("Skill '{}' 不在待更新列表中", dir_name),
            },
            AppError::InvalidInput { input, message } => AppErrorDto {
                code: "invalidInput".to_string(),
                message: format!("无法识别链接格式：{} ({})", input, message),
            },
            AppError::MissingSkillPath { input } => AppErrorDto {
                code: "missingSkillPath".to_string(),
                message: format!(
                    "链接「{}」只包含仓库信息，请粘贴指向 Skill 目录或 SKILL.md 的 GitHub 地址。示例：{}",
                    input.trim(),
                    SMART_PASTE_GITHUB_EXAMPLE
                ),
            },
            AppError::SkillRepoNotFound { owner, name } => AppErrorDto {
                code: "skillRepoNotFound".to_string(),
                message: format!("找不到 Skill 仓库 {}/{}", owner, name),
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
            AppError::DownloadFailed {
                url,
                status,
                message,
            } => {
                if let Some(status) = status {
                    write!(
                        formatter,
                        "download failed for {} (HTTP {}): {}",
                        url, status, message
                    )
                } else {
                    write!(formatter, "download failed for {}: {}", url, message)
                }
            }
            AppError::DiscoverInProgress => {
                write!(formatter, "skill discovery already in progress")
            }
            AppError::DirExists { path } => {
                write!(formatter, "directory already exists at {}", path.display())
            }
            AppError::SkillDirNotFound { path } => {
                write!(formatter, "skill directory not found at {}", path.display())
            }
            AppError::UpdatesInProgress => {
                write!(formatter, "skill update check already in progress")
            }
            AppError::UpdateNotPending { dir_name } => {
                write!(formatter, "skill '{}' is not pending update", dir_name)
            }
            AppError::InvalidInput { input, message } => {
                write!(formatter, "invalid input '{}': {}", input, message)
            }
            AppError::MissingSkillPath { input } => {
                write!(formatter, "missing skill path in input '{}'", input)
            }
            AppError::SkillRepoNotFound { owner, name } => {
                write!(formatter, "skill repo not found: {}/{}", owner, name)
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
