use crate::models::{AppError, TargetScope};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPreset {
    pub id: String,
    pub display_name: String,
    pub global_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_relative_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserPresetsFile {
    presets: Vec<AgentPreset>,
}

pub fn builtin_presets() -> Vec<AgentPreset> {
    vec![
        AgentPreset {
            id: "cursor".to_string(),
            display_name: "Cursor".to_string(),
            global_path: "~/.cursor/skills".to_string(),
            project_relative_path: Some(".cursor/skills".to_string()),
            icon: Some("cursor.png".to_string()),
        },
        AgentPreset {
            id: "claude".to_string(),
            display_name: "Claude Code".to_string(),
            global_path: "~/.claude/skills".to_string(),
            project_relative_path: Some(".claude/skills".to_string()),
            icon: Some("claude_code.svg".to_string()),
        },
        AgentPreset {
            id: "codex".to_string(),
            display_name: "Codex".to_string(),
            global_path: "~/.codex/skills".to_string(),
            project_relative_path: Some(".codex/skills".to_string()),
            icon: Some("codex.svg".to_string()),
        },
    ]
}

pub fn load_merged_presets(app_data_dir: &Path) -> Result<Vec<AgentPreset>, AppError> {
    let mut merged: HashMap<String, AgentPreset> = builtin_presets()
        .into_iter()
        .map(|preset| (preset.id.clone(), preset))
        .collect();

    let user_path = app_data_dir.join("agent-presets.json");
    if user_path.is_file() {
        let raw = std::fs::read_to_string(&user_path).map_err(|error| AppError::ConfigRead {
            path: user_path.clone(),
            message: error.to_string(),
        })?;
        let user_file: UserPresetsFile =
            serde_json::from_str(&raw).map_err(|error| AppError::ConfigParse {
                path: user_path,
                message: error.to_string(),
            })?;
        for preset in user_file.presets {
            merged.insert(preset.id.clone(), preset);
        }
    }

    Ok(merged.into_values().collect())
}

pub fn presets_for_scope(presets: &[AgentPreset], scope: TargetScope) -> Vec<&AgentPreset> {
    match scope {
        TargetScope::Global => presets.iter().collect(),
        TargetScope::Project => presets
            .iter()
            .filter(|preset| preset.project_relative_path.is_some())
            .collect(),
    }
}

pub fn resolve_skills_dir(
    preset: &AgentPreset,
    scope: TargetScope,
    project_root: Option<&Path>,
) -> Result<PathBuf, AppError> {
    match scope {
        TargetScope::Global => expand_tilde(&preset.global_path),
        TargetScope::Project => {
            let relative = preset.project_relative_path.as_ref().ok_or_else(|| {
                AppError::InvalidInput {
                    input: preset.id.clone(),
                    message: "preset has no project-relative path".to_string(),
                }
            })?;
            let root = project_root.ok_or_else(|| AppError::InvalidInput {
                input: preset.id.clone(),
                message: "project root is required for project scope".to_string(),
            })?;
            Ok(normalize_platform_path(root.join(relative)))
        }
    }
}

pub fn normalize_path_for_compare(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    {
        return normalized.to_lowercase();
    }
    #[cfg(not(windows))]
    {
        normalized
    }
}

/// Rebuild a path with the current platform's native separators.
pub fn normalize_platform_path(path: impl AsRef<Path>) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.as_ref().components() {
        normalized.push(component.as_os_str());
    }
    normalized
}

pub fn detect_agent_id_for_path(path: &Path) -> Option<String> {
    let normalized_path = normalize_path_for_compare(path);
    for preset in builtin_presets() {
        let expanded = expand_tilde(&preset.global_path).ok()?;
        if normalize_path_for_compare(&expanded) == normalized_path {
            return Some(preset.id);
        }
    }
    None
}

/// Infer a project root from a legacy v0.4 target skills directory.
pub fn infer_project_root_from_skills_dir(skills_dir: &Path) -> PathBuf {
    let normalized = normalize_path_for_compare(skills_dir);

    for preset in builtin_presets() {
        let Some(relative) = preset.project_relative_path.as_ref() else {
            continue;
        };
        let relative_norm = relative.replace('\\', "/").trim_start_matches('/').to_lowercase();
        let suffix = format!("/{relative_norm}");
        if normalized.ends_with(&suffix) || normalized == relative_norm {
            let mut root = skills_dir.to_path_buf();
            for _ in relative.split('/').filter(|part| !part.is_empty()) {
                root.pop();
            }
            return root;
        }
    }

    if skills_dir.file_name().and_then(|name| name.to_str()) == Some("skills") {
        if let Some(agent_dir) = skills_dir.parent() {
            if agent_dir
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('.'))
            {
                if let Some(root) = agent_dir.parent() {
                    if !is_likely_filesystem_root(root) {
                        return root.to_path_buf();
                    }
                }
            } else if !is_likely_filesystem_root(agent_dir) {
                return agent_dir.to_path_buf();
            }
        }
    }

    skills_dir.to_path_buf()
}

/// Infer a display name from `*/.<agent>/skills` paths, e.g. `.trae/skills` → `trae`.
pub fn infer_target_name_from_skills_dir(skills_dir: &Path) -> Option<String> {
    if skills_dir.file_name().and_then(|name| name.to_str()) != Some("skills") {
        return None;
    }
    let agent_dir = skills_dir.parent()?;
    let folder = agent_dir.file_name()?.to_str()?;
    folder
        .strip_prefix('.')
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
}

fn is_likely_filesystem_root(path: &Path) -> bool {
    match path.parent() {
        None => true,
        Some(parent) => {
            let parent_str = parent.to_string_lossy();
            if parent_str.is_empty() {
                return true;
            }
            #[cfg(windows)]
            {
                let trimmed = parent_str.trim_end_matches(['\\', '/']);
                trimmed.len() <= 2 && trimmed.contains(':')
            }
            #[cfg(not(windows))]
            {
                parent.as_os_str().is_empty() || parent == Path::new("/")
            }
        }
    }
}

fn expand_tilde(path_template: &str) -> Result<PathBuf, AppError> {
    if path_template == "~" {
        return home_dir().ok_or_else(|| AppError::Io {
            path: None,
            message: "home directory is not available".to_string(),
        });
    }

    if let Some(rest) = path_template.strip_prefix("~/") {
        let home = home_dir().ok_or_else(|| AppError::Io {
            path: None,
            message: "home directory is not available".to_string(),
        })?;
        return Ok(normalize_platform_path(home.join(rest)));
    }

    Ok(normalize_platform_path(PathBuf::from(path_template)))
}

pub(crate) fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn builtin_presets_include_cursor_claude_codex() {
        let presets = builtin_presets();
        let ids: Vec<_> = presets.iter().map(|preset| preset.id.as_str()).collect();
        assert_eq!(ids, vec!["cursor", "claude", "codex"]);
    }

    #[test]
    fn list_for_project_scope_excludes_missing_project_relative_path() {
        let mut presets = builtin_presets();
        presets.push(AgentPreset {
            id: "opencode".to_string(),
            display_name: "OpenCode".to_string(),
            global_path: "~/.opencode/skills".to_string(),
            project_relative_path: None,
            icon: None,
        });

        let global = presets_for_scope(&presets, TargetScope::Global);
        assert_eq!(global.len(), 4);

        let project = presets_for_scope(&presets, TargetScope::Project);
        assert_eq!(project.len(), 3);
        assert!(project.iter().all(|preset| preset.id != "opencode"));
    }

    #[test]
    fn merge_user_presets_overrides_by_id() {
        let temp = TempDir::new().expect("temp dir");
        let user_path = temp.path().join("agent-presets.json");
        fs::write(
            &user_path,
            r#"{
                "presets": [
                    {
                        "id": "cursor",
                        "displayName": "My Cursor",
                        "globalPath": "~/.cursor/skills",
                        "projectRelativePath": ".cursor/skills"
                    }
                ]
            }"#,
        )
        .expect("write user presets");

        let merged = load_merged_presets(temp.path()).expect("load merged presets");
        let cursor = merged
            .iter()
            .find(|preset| preset.id == "cursor")
            .expect("cursor preset");
        assert_eq!(cursor.display_name, "My Cursor");
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn resolve_global_path_expands_tilde() {
        let preset = builtin_presets()
            .into_iter()
            .find(|preset| preset.id == "cursor")
            .expect("cursor preset");
        let resolved =
            resolve_skills_dir(&preset, TargetScope::Global, None).expect("resolve global path");
        let home = home_dir().expect("home dir");
        assert_eq!(resolved, normalize_platform_path(home.join(".cursor/skills")));
    }

    #[test]
    fn normalize_platform_path_unifies_mixed_separators() {
        if cfg!(windows) {
            let mixed = PathBuf::from(r"C:\Users\demo\.codex/skills");
            let normalized = normalize_platform_path(&mixed);
            assert!(!normalized.to_string_lossy().contains('/'));
            assert!(normalized.ends_with("skills"));
        } else {
            let mixed = PathBuf::from("/home/demo/.codex\\skills");
            let normalized = normalize_platform_path(&mixed);
            assert!(!normalized.to_string_lossy().contains('\\'));
            assert!(normalized.ends_with("skills"));
        }
    }

    #[test]
    fn infer_project_root_from_dot_agent_skills_path() {
        let root = infer_project_root_from_skills_dir(Path::new("C:/Git/efs/.trae/skills"));
        assert_eq!(root, PathBuf::from("C:/Git/efs"));
    }

    #[test]
    fn infer_target_name_from_dot_agent_skills_path() {
        assert_eq!(
            infer_target_name_from_skills_dir(Path::new("C:/Git/efs/.trae/skills")).as_deref(),
            Some("trae")
        );
        assert_eq!(
            infer_target_name_from_skills_dir(Path::new("C:/repo/.cursor/skills")).as_deref(),
            Some("cursor")
        );
        assert_eq!(
            infer_target_name_from_skills_dir(Path::new("D:/skills")),
            None
        );
    }

    #[test]
    fn normalize_path_matches_migration_detect() {
        let home = home_dir().expect("home dir");
        let canonical = home.join(".cursor").join("skills");
        let forward_slashes =
            PathBuf::from(canonical.to_string_lossy().replace('\\', "/"));
        #[cfg(windows)]
        let different_case = PathBuf::from(canonical.to_string_lossy().to_uppercase());
        #[cfg(not(windows))]
        let different_case = forward_slashes.clone();

        assert_eq!(
            normalize_path_for_compare(&canonical),
            normalize_path_for_compare(&forward_slashes)
        );
        assert_eq!(
            detect_agent_id_for_path(&canonical),
            Some("cursor".to_string())
        );
        assert_eq!(
            detect_agent_id_for_path(&different_case),
            Some("cursor".to_string())
        );
    }
}
