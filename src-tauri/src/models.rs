use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Current on-disk config schema version. Bump when adding breaking fields.
pub const CURRENT_CONFIG_VERSION: u32 = 6;

pub(crate) fn default_github_host() -> String {
    "github.com".to_string()
}

pub(crate) fn default_github_provider() -> String {
    "github".to_string()
}

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
    #[serde(default)]
    pub gitlab_credential_hosts: Vec<String>,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub skill_hub_endpoints: Vec<SkillHubEndpoint>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_CONFIG_VERSION,
            settings: Settings::default(),
            targets: Vec::new(),
            installations: Vec::new(),
            skill_repos: Vec::new(),
            skill_records: HashMap::new(),
            skill_discover_cache: SkillDiscoverCache::default(),
            skill_update_cache: SkillUpdateCache::default(),
            gitlab_credential_hosts: Vec::new(),
            projects: Vec::new(),
            skill_hub_endpoints: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillHubEndpoint {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
}

/// Upgrade an on-disk config to the current schema. Returns true when persisted state changed.
pub fn migrate_config(config: &mut AppConfig) -> bool {
    if config.version >= CURRENT_CONFIG_VERSION {
        return false;
    }

    if config.version < 4 {
        for repo in &mut config.skill_repos {
            repo.host = default_github_host();
            repo.provider = default_github_provider();
            if repo.project_path.is_empty() {
                repo.project_path = format!("{}/{}", repo.owner, repo.name);
            }
        }
        for record in config.skill_records.values_mut() {
            if record.repo_host.is_empty() {
                record.repo_host = default_github_host();
            }
            if record.project_path.is_empty() {
                record.project_path = format!("{}/{}", record.repo_owner, record.repo_name);
            }
        }
    }

    if config.version < 5 {
        migrate_v4_to_v5(config);
        config.version = 5;
        return true;
    }

    // v5→v6 filesystem migration is handled in config_store before this is called again.
    false
}

/// Normalize stored filesystem paths to native platform separators.
pub fn normalize_config_paths(config: &mut AppConfig) -> bool {
    use crate::agent_presets::normalize_platform_path;

    let mut changed = false;

    if let Some(dir) = config.settings.main_skills_dir.as_mut() {
        let normalized = normalize_platform_path(dir.as_path());
        if *dir != normalized {
            *dir = normalized;
            changed = true;
        }
    }

    for target in &mut config.targets {
        let normalized = normalize_platform_path(&target.skills_dir);
        if target.skills_dir != normalized {
            target.skills_dir = normalized;
            changed = true;
        }
        if let Some(path) = target.custom_path.as_mut() {
            let normalized = normalize_platform_path(path.as_path());
            if *path != normalized {
                *path = normalized;
                changed = true;
            }
        }
    }

    for project in &mut config.projects {
        let normalized = normalize_platform_path(&project.root_path);
        if project.root_path != normalized {
            project.root_path = normalized;
            changed = true;
        }
    }

    for installation in &mut config.installations {
        let source = normalize_platform_path(&installation.source_path);
        if installation.source_path != source {
            installation.source_path = source;
            changed = true;
        }
        let link = normalize_platform_path(&installation.link_path);
        if installation.link_path != link {
            installation.link_path = link;
            changed = true;
        }
    }

    changed
}

fn migrate_v4_to_v5(config: &mut AppConfig) {
    use std::collections::HashMap;

    let mut root_to_project_id: HashMap<String, String> = HashMap::new();

    for target in &mut config.targets {
        if target.custom_path.is_none() {
            target.custom_path = Some(target.skills_dir.clone());
        }

        if let Some(agent_id) = crate::agent_presets::detect_agent_id_for_path(&target.skills_dir)
        {
            if let Some(preset) = crate::agent_presets::builtin_presets()
                .into_iter()
                .find(|preset| preset.id == agent_id)
            {
                target.scope = TargetScope::Global;
                target.kind = TargetKind::Agent;
                target.agent_id = Some(agent_id);
                target.project_id = None;
                target.custom_path = None;
                target.name = preset.display_name;
                continue;
            }
        }

        let root_path =
            crate::agent_presets::infer_project_root_from_skills_dir(&target.skills_dir);
        let root_key = crate::agent_presets::normalize_path_for_compare(&root_path);
        let project_name = if root_path == target.skills_dir {
            target.name.clone()
        } else {
            root_path
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .unwrap_or(target.name.as_str())
                .to_string()
        };
        let project_id = root_to_project_id.get(&root_key).cloned().unwrap_or_else(|| {
            let id = format!("project-{}", target.id);
            config.projects.push(Project {
                id: id.clone(),
                name: project_name,
                root_path: root_path.clone(),
                created_at: target.created_at.clone(),
                updated_at: target.updated_at.clone(),
            });
            root_to_project_id.insert(root_key, id.clone());
            id
        });

        target.scope = TargetScope::Project;
        target.kind = TargetKind::Custom;
        target.agent_id = None;
        target.project_id = Some(project_id);
        target.custom_path = Some(target.skills_dir.clone());
        if let Some(name) =
            crate::agent_presets::infer_target_name_from_skills_dir(&target.skills_dir)
        {
            target.name = name;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartupRefreshSettings {
    #[serde(default)]
    pub github: bool,
    #[serde(default = "default_startup_refresh_enabled")]
    pub gitlab: bool,
    #[serde(default = "default_startup_refresh_enabled")]
    pub skill_hub: bool,
}

fn default_startup_refresh_enabled() -> bool {
    true
}

impl Default for StartupRefreshSettings {
    fn default() -> Self {
        Self {
            github: false,
            gitlab: true,
            skill_hub: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub main_skills_dir: Option<PathBuf>,
    pub link_strategy: LinkStrategy,
    #[serde(default)]
    pub startup_refresh: StartupRefreshSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            main_skills_dir: None,
            link_strategy: LinkStrategy::Auto,
            startup_refresh: StartupRefreshSettings::default(),
        }
    }
}

/// Link creation strategy. Currently only `Auto` is implemented (OS-appropriate
/// junction/symlink); additional variants are reserved for future use.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinkStrategy {
    Auto,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum TargetScope {
    #[default]
    Global,
    Project,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum TargetKind {
    Agent,
    #[default]
    Custom,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: PathBuf,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub scope: TargetScope,
    #[serde(default)]
    pub kind: TargetKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_path: Option<PathBuf>,
    pub skills_dir: PathBuf,
    pub created_at: String,
    pub updated_at: String,
}

impl Target {
    pub fn global_custom(
        id: impl Into<String>,
        name: impl Into<String>,
        skills_dir: PathBuf,
        created_at: impl Into<String>,
        updated_at: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            scope: TargetScope::Global,
            kind: TargetKind::Custom,
            agent_id: None,
            project_id: None,
            custom_path: Some(skills_dir.clone()),
            skills_dir,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
        }
    }
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
    #[serde(default)]
    pub skill_storage_key: String,
}

impl Default for Installation {
    fn default() -> Self {
        Self {
            id: String::new(),
            skill_dir_name: String::new(),
            skill_name: String::new(),
            source_path: PathBuf::new(),
            target_id: String::new(),
            link_path: PathBuf::new(),
            link_type: LinkType::Junction,
            created_at: String::new(),
            skill_storage_key: String::new(),
        }
    }
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
    #[serde(default)]
    pub storage_key: String,
    #[serde(default)]
    pub link_name: String,
    #[serde(default)]
    pub local_dirty: bool,
}

impl Default for SkillView {
    fn default() -> Self {
        Self {
            dir_name: String::new(),
            name: None,
            description: None,
            path: PathBuf::new(),
            valid: false,
            validation_errors: Vec::new(),
            storage_key: String::new(),
            link_name: String::new(),
            local_dirty: false,
        }
    }
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
pub struct SkillMarkdownPreviewDto {
    pub title: String,
    pub description: String,
    pub markdown_body: String,
    pub origin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SkillMarkdownRequestDto {
    #[serde(rename_all = "camelCase")]
    Installed { storage_key: String },
    #[serde(rename_all = "camelCase")]
    Discover { discover_key: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRepo {
    #[serde(default = "default_github_host")]
    pub host: String,
    #[serde(default = "default_github_provider")]
    pub provider: String,
    #[serde(default)]
    pub project_path: String,
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub enabled: bool,
}

impl SkillRepo {
    pub fn to_repo_ref(&self) -> RepoRef {
        let project_path = if self.project_path.is_empty() {
            format!("{}/{}", self.owner, self.name)
        } else {
            self.project_path.clone()
        };
        RepoRef {
            host: self.host.clone(),
            provider: self.provider.clone(),
            project_path,
            branch: self.branch.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRef {
    pub host: String,
    pub provider: String,
    pub project_path: String,
    pub branch: String,
}

impl DiscoverableSkill {
    pub fn to_repo_ref(&self) -> RepoRef {
        let project_path = if self.project_path.is_empty() {
            format!("{}/{}", self.repo_owner, self.repo_name)
        } else {
            self.project_path.clone()
        };
        RepoRef {
            host: if self.repo_host.is_empty() {
                default_github_host()
            } else {
                self.repo_host.clone()
            },
            provider: self.source.clone(),
            project_path,
            branch: self.repo_branch.clone(),
        }
    }
}

impl SkillRecord {
    pub fn to_repo_ref(&self) -> RepoRef {
        let project_path = if self.project_path.is_empty() {
            format!("{}/{}", self.repo_owner, self.repo_name)
        } else {
            self.project_path.clone()
        };
        let provider = if self.source == "gitlab" || self.repo_host != default_github_host() {
            "gitlab".to_string()
        } else {
            default_github_provider()
        };
        RepoRef {
            host: if self.repo_host.is_empty() {
                default_github_host()
            } else {
                self.repo_host.clone()
            },
            provider,
            project_path,
            branch: self.repo_branch.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
    #[serde(default = "default_github_host")]
    pub repo_host: String,
    #[serde(default)]
    pub project_path: String,
    pub source: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub directory: String,
    pub content_hash: String,
    pub installed_at: String,
    #[serde(default)]
    pub storage_key: String,
    #[serde(default)]
    pub link_name: String,
    #[serde(default)]
    pub repo_slug: String,
    #[serde(default)]
    pub hub_endpoint_id: String,
    #[serde(default)]
    pub hub_skill_group: String,
    #[serde(default)]
    pub hub_skill_id: String,
    /// Hub/remote source no longer has this skill; local copy may still exist.
    #[serde(default)]
    pub source_missing: bool,
}

impl Default for SkillRecord {
    fn default() -> Self {
        Self {
            repo_host: default_github_host(),
            project_path: String::new(),
            source: String::new(),
            repo_owner: String::new(),
            repo_name: String::new(),
            repo_branch: String::new(),
            directory: String::new(),
            content_hash: String::new(),
            installed_at: String::new(),
            storage_key: String::new(),
            link_name: String::new(),
            repo_slug: String::new(),
            hub_endpoint_id: String::new(),
            hub_skill_group: String::new(),
            hub_skill_id: String::new(),
            source_missing: false,
        }
    }
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

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverSkillsResult {
    pub skills: Vec<DiscoverableSkill>,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverableSkill {
    pub key: String,
    pub name: String,
    pub description: String,
    pub directory: String,
    pub install_dir_name: String,
    #[serde(default = "default_github_host")]
    pub repo_host: String,
    #[serde(default)]
    pub project_path: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub source: String,
    #[serde(default)]
    pub storage_key: String,
    #[serde(default)]
    pub link_name: String,
    #[serde(default)]
    pub repo_slug: String,
    #[serde(default)]
    pub hub_endpoint_id: String,
    #[serde(default)]
    pub hub_skill_group: String,
    #[serde(default)]
    pub hub_skill_id: String,
}

impl Default for DiscoverableSkill {
    fn default() -> Self {
        Self {
            key: String::new(),
            name: String::new(),
            description: String::new(),
            directory: String::new(),
            install_dir_name: String::new(),
            repo_host: default_github_host(),
            project_path: String::new(),
            repo_owner: String::new(),
            repo_name: String::new(),
            repo_branch: String::new(),
            source: String::new(),
            storage_key: String::new(),
            link_name: String::new(),
            repo_slug: String::new(),
            hub_endpoint_id: String::new(),
            hub_skill_group: String::new(),
            hub_skill_id: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRepoChangeResult {
    pub repos: Vec<SkillRepo>,
    pub discover_skills: Vec<DiscoverableSkill>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillHubEndpointChangeResult {
    pub endpoints: Vec<SkillHubEndpoint>,
    pub discover_skills: Vec<DiscoverableSkill>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAddRepoResult {
    pub can_save: bool,
    pub needs_pat: bool,
    pub host: Option<String>,
    pub provider: Option<String>,
    pub project_path: Option<String>,
    pub branch: Option<String>,
    pub error: Option<AppErrorDto>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateInfo {
    pub dir_name: String,
    pub name: String,
    pub current_hash: Option<String>,
    pub remote_hash: String,
    #[serde(default)]
    pub storage_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartupSkillRefreshResult {
    pub discover_skills: Vec<DiscoverableSkill>,
    pub pending_updates: Vec<SkillUpdateInfo>,
    pub warnings: Vec<String>,
}

impl Default for SkillUpdateInfo {
    fn default() -> Self {
        Self {
            dir_name: String::new(),
            name: String::new(),
            current_hash: None,
            remote_hash: String::new(),
            storage_key: String::new(),
        }
    }
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
    #[serde(default = "default_github_host")]
    pub repo_host: String,
    #[serde(default)]
    pub project_path: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub directory: String,
    pub source: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPresetDto {
    pub id: String,
    pub display_name: String,
    pub global_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_relative_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub version: String,
    pub current_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReportDto {
    pub backed_up_config: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backed_up_main: Option<String>,
    pub succeeded: Vec<String>,
    pub failed: Vec<String>,
    pub orphan_locals: Vec<String>,
    pub links_repaired: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncInstallFailure {
    pub storage_key: String,
    pub label: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncTargetInstallationsResponse {
    pub installed: u32,
    pub skipped: u32,
    pub failed: Vec<SyncInstallFailure>,
    pub state: AppState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub config: AppConfig,
    pub skills: Vec<SkillView>,
    pub selected_target_id: Option<String>,
    pub selected_target_skills: Vec<SkillWithTargetState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_migration_report: Option<MigrationReportDto>,
    /// When false, `skills` may be empty/omitted by the client merge layer.
    /// Defaults to true for backward compatibility.
    #[serde(default = "default_true")]
    pub skills_included: bool,
    /// Soft warnings from the last force-cleanup (target/project/skill).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cleanup_warnings: Vec<String>,
}

fn default_true() -> bool {
    true
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
    /// Hub (or similar) reported the skill no longer exists (HTTP 404).
    HubSkillGone {
        skill_id: String,
        group: String,
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
    CredentialStore {
        message: String,
    },
    GitLabAuthRequired {
        host: String,
    },
    GitLabAuthInvalid {
        host: String,
    },
    ProjectNotFound {
        project_id: String,
    },
    DuplicateProjectName {
        name: String,
    },
    InvalidProjectRoot {
        path: PathBuf,
        message: String,
    },
    DuplicateTarget {
        message: String,
    },
    TargetNotEditable {
        target_id: String,
    },
    ProjectHasTargetsWithInstallations {
        project_id: String,
        installation_count: usize,
    },
    PathOutsideProjectRoot {
        path: PathBuf,
        project_root: PathBuf,
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
                message: if *status == Some(403) && message.contains("请稍后再试") {
                    message.clone()
                } else {
                    match status {
                        Some(code) => format!("下载失败 {} (HTTP {}): {}", url, code, message),
                        None => format!("下载失败 {}: {}", url, message),
                    }
                },
            },
            AppError::HubSkillGone { skill_id, group } => AppErrorDto {
                code: "hubSkillGone".to_string(),
                message: format!(
                    "源中已不存在「{}」（分组 {}）",
                    skill_id, group
                ),
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
            AppError::CredentialStore { message } => AppErrorDto {
                code: "credentialStore".to_string(),
                message: message.clone(),
            },
            AppError::GitLabAuthRequired { host } => AppErrorDto {
                code: "gitlabAuthRequired".to_string(),
                message: format!(
                    "访问 GitLab {} 需要登录，请先配置 Personal Access Token",
                    host
                ),
            },
            AppError::GitLabAuthInvalid { host } => AppErrorDto {
                code: "gitlabAuthInvalid".to_string(),
                message: format!("GitLab {} 的 Token 无效或已过期，请重新配置", host),
            },
            AppError::ProjectNotFound { project_id } => AppErrorDto {
                code: "projectNotFound".to_string(),
                message: format!("找不到项目 {}", project_id),
            },
            AppError::DuplicateProjectName { name } => AppErrorDto {
                code: "duplicateProjectName".to_string(),
                message: format!("项目名称已存在：{}", name),
            },
            AppError::InvalidProjectRoot { path, message } => AppErrorDto {
                code: "invalidProjectRoot".to_string(),
                message: format!("项目根目录无效 {}: {}", path.display(), message),
            },
            AppError::DuplicateTarget { message } => AppErrorDto {
                code: "duplicateTarget".to_string(),
                message: message.clone(),
            },
            AppError::TargetNotEditable { target_id } => AppErrorDto {
                code: "targetNotEditable".to_string(),
                message: format!("目标 {} 不可编辑", target_id),
            },
            AppError::ProjectHasTargetsWithInstallations {
                project_id,
                installation_count,
            } => AppErrorDto {
                code: "projectHasTargetsWithInstallations".to_string(),
                message: format!(
                    "项目 {} 下仍有 {} 条安装记录，请先卸载",
                    project_id, installation_count
                ),
            },
            AppError::PathOutsideProjectRoot { path, project_root } => AppErrorDto {
                code: "pathOutsideProjectRoot".to_string(),
                message: format!(
                    "路径 {} 不在项目根目录 {} 内",
                    path.display(),
                    project_root.display()
                ),
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
                if *status == Some(403) && message.contains("请稍后再试") {
                    write!(formatter, "{}", message)
                } else if let Some(status) = status {
                    write!(
                        formatter,
                        "download failed for {} (HTTP {}): {}",
                        url, status, message
                    )
                } else {
                    write!(formatter, "download failed for {}: {}", url, message)
                }
            }
            AppError::HubSkillGone { skill_id, group } => {
                write!(
                    formatter,
                    "hub skill gone: {}/{}",
                    group, skill_id
                )
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
            AppError::CredentialStore { message } => {
                write!(formatter, "credential store error: {}", message)
            }
            AppError::GitLabAuthRequired { host } => {
                write!(formatter, "gitlab authentication required for {}", host)
            }
            AppError::GitLabAuthInvalid { host } => {
                write!(formatter, "gitlab authentication invalid for {}", host)
            }
            AppError::ProjectNotFound { project_id } => {
                write!(formatter, "project not found: {}", project_id)
            }
            AppError::DuplicateProjectName { name } => {
                write!(formatter, "duplicate project name: {}", name)
            }
            AppError::InvalidProjectRoot { path, message } => {
                write!(
                    formatter,
                    "invalid project root at {}: {}",
                    path.display(),
                    message
                )
            }
            AppError::DuplicateTarget { message } => {
                write!(formatter, "duplicate target: {}", message)
            }
            AppError::TargetNotEditable { target_id } => {
                write!(formatter, "target not editable: {}", target_id)
            }
            AppError::ProjectHasTargetsWithInstallations {
                project_id,
                installation_count,
            } => write!(
                formatter,
                "project {} still has {} installation record(s) on child targets",
                project_id, installation_count
            ),
            AppError::PathOutsideProjectRoot { path, project_root } => write!(
                formatter,
                "path {} is outside project root {}",
                path.display(),
                project_root.display()
            ),
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
            skill_storage_key: String::new(),
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

    #[test]
    fn migrate_v4_agent_path_becomes_agent_target() {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .expect("home dir");
        let skills_dir = home.join(".cursor").join("skills");
        let mut config = AppConfig {
            version: 4,
            targets: vec![Target {
                id: "t1".into(),
                name: "Old Name".into(),
                scope: TargetScope::Global,
                kind: TargetKind::Custom,
                agent_id: None,
                project_id: None,
                custom_path: None,
                skills_dir: skills_dir.clone(),
                created_at: "1".into(),
                updated_at: "1".into(),
            }],
            ..Default::default()
        };

        assert!(migrate_config(&mut config));
        assert_eq!(config.targets[0].kind, TargetKind::Agent);
        assert_eq!(config.targets[0].agent_id.as_deref(), Some("cursor"));
        assert_eq!(config.targets[0].name, "Cursor");
        assert_eq!(config.targets[0].custom_path, None);
        assert_eq!(config.targets[0].skills_dir, skills_dir);
    }

    #[test]
    fn migrate_v4_json_adds_scope_and_projects() {
        let raw = r#"{"version":4,"settings":{"mainSkillsDir":null,"linkStrategy":"auto"},"targets":[{"id":"t1","name":"X","skillsDir":"D:/skills","createdAt":"1","updatedAt":"1"}],"installations":[]}"#;
        let mut config: AppConfig = serde_json::from_str(raw).unwrap();
        assert!(migrate_config(&mut config));
        assert_eq!(config.version, 5);
        assert_eq!(config.projects.len(), 1);
        assert_eq!(config.projects[0].name, "X");
        assert_eq!(config.projects[0].root_path, PathBuf::from("D:/skills"));
        assert_eq!(config.targets[0].scope, TargetScope::Project);
        assert_eq!(config.targets[0].kind, TargetKind::Custom);
        assert_eq!(config.targets[0].project_id.as_deref(), Some("project-t1"));
        assert_eq!(
            config.targets[0].custom_path,
            Some(PathBuf::from("D:/skills"))
        );
    }

    #[test]
    fn migrate_v4_non_agent_target_becomes_project_scoped() {
        let raw = r#"{"version":4,"settings":{"mainSkillsDir":null,"linkStrategy":"auto"},"targets":[{"id":"t1","name":"efs","skillsDir":"C:/Git/efs/.trae/skills","createdAt":"1","updatedAt":"1"}],"installations":[]}"#;
        let mut config: AppConfig = serde_json::from_str(raw).unwrap();
        assert!(migrate_config(&mut config));
        assert_eq!(config.projects.len(), 1);
        assert_eq!(config.projects[0].name, "efs");
        assert_eq!(config.targets[0].name, "trae");
        assert_eq!(
            config.projects[0].root_path,
            PathBuf::from("C:/Git/efs")
        );
        assert_eq!(config.targets[0].scope, TargetScope::Project);
        assert_eq!(config.targets[0].project_id.as_deref(), Some("project-t1"));
        assert_eq!(
            config.targets[0].skills_dir,
            PathBuf::from("C:/Git/efs/.trae/skills")
        );
    }

    #[test]
    fn skill_record_v6_fields_deserialize_with_defaults() {
        let raw = r#"{
        "source": "github",
        "repoOwner": "anthropics",
        "repoName": "skills",
        "repoBranch": "main",
        "directory": "skills/tdd",
        "contentHash": "abc",
        "installedAt": "2026-01-01T00:00:00Z"
    }"#;
        let record: SkillRecord = serde_json::from_str(raw).expect("parse v5 record");
        assert_eq!(record.storage_key, "");
        assert_eq!(record.link_name, "");
    }

    #[test]
    fn app_config_deserializes_skill_hub_endpoints_default_empty() {
        let raw = r#"{"version":5,"settings":{"linkStrategy":"auto"},"targets":[],"installations":[]}"#;
        let config: AppConfig = serde_json::from_str(raw).expect("parse");
        assert!(config.skill_hub_endpoints.is_empty());
    }

    #[test]
    fn settings_default_startup_refresh_prefers_internal_sources() {
        let settings = Settings::default();
        assert!(!settings.startup_refresh.github);
        assert!(settings.startup_refresh.gitlab);
        assert!(settings.startup_refresh.skill_hub);
    }

    #[test]
    fn old_settings_json_gets_startup_refresh_defaults() {
        let raw = r#"{"mainSkillsDir":null,"linkStrategy":"auto"}"#;
        let settings: Settings = serde_json::from_str(raw).expect("parse old settings");
        assert_eq!(
            settings.startup_refresh,
            StartupRefreshSettings::default()
        );
    }

    #[test]
    fn startup_refresh_settings_round_trip() {
        let settings = Settings {
            startup_refresh: StartupRefreshSettings {
                github: true,
                gitlab: false,
                skill_hub: true,
            },
            ..Settings::default()
        };
        let raw = serde_json::to_string(&settings).expect("serialize settings");
        let restored: Settings = serde_json::from_str(&raw).expect("deserialize settings");
        assert_eq!(restored, settings);
    }

    #[test]
    fn migrate_config_v3_to_v4_fills_repo_fields() {
        let json = r#"{
            "version": 3,
            "settings": { "mainSkillsDir": null, "linkStrategy": "auto" },
            "targets": [],
            "installations": [],
            "skillRepos": [{ "owner": "obra", "name": "superpowers", "branch": "main", "enabled": true }],
            "skillRecords": {},
            "skillDiscoverCache": { "skills": [] },
            "skillUpdateCache": { "updates": [] }
        }"#;
        let mut config: AppConfig = serde_json::from_str(json).unwrap();
        assert!(migrate_config(&mut config));
        assert_eq!(config.version, 5);
        let repo = &config.skill_repos[0];
        assert_eq!(repo.host, "github.com");
        assert_eq!(repo.provider, "github");
        assert_eq!(repo.project_path, "obra/superpowers");
    }
}
