use crate::models::{
    default_github_host, AppConfig, DiscoverableSkill, SkillRecord, SkillUpdateInfo,
    StartupSkillRefreshResult,
};
use crate::skill_discover::deduplicate_discoverable_skills;
use std::collections::HashSet;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Github,
    Gitlab,
    SkillHub,
    IflytekSkillHub,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreshSourceResult {
    pub discover_skills: Vec<DiscoverableSkill>,
    pub pending_updates: Vec<SkillUpdateInfo>,
}

fn enabled_source_kinds(config: &AppConfig) -> Vec<SourceKind> {
    let settings = &config.settings.startup_refresh;
    let mut kinds = Vec::new();
    if settings.github {
        kinds.push(SourceKind::Github);
    }
    if settings.gitlab {
        kinds.push(SourceKind::Gitlab);
    }
    if settings.skill_hub {
        kinds.push(SourceKind::SkillHub);
    }
    if settings.iflytek_skill_hub {
        kinds.push(SourceKind::IflytekSkillHub);
    }
    kinds
}

fn record_source_kind(record: &SkillRecord) -> SourceKind {
    if record.source == "skillhub" {
        SourceKind::SkillHub
    } else if record.source == "iflytek" {
        SourceKind::IflytekSkillHub
    } else if record.source == "gitlab" || record.repo_host != default_github_host() {
        SourceKind::Gitlab
    } else {
        SourceKind::Github
    }
}

pub fn discover_source_kind(skill: &DiscoverableSkill) -> SourceKind {
    if skill.source == "skillhub" {
        SourceKind::SkillHub
    } else if skill.source == "iflytek" {
        SourceKind::IflytekSkillHub
    } else if skill.source == "gitlab" || skill.repo_host != default_github_host() {
        SourceKind::Gitlab
    } else {
        SourceKind::Github
    }
}

pub fn update_source_kind(
    config: &AppConfig,
    update: &SkillUpdateInfo,
) -> Option<SourceKind> {
    config
        .skill_records
        .iter()
        .find(|(record_key, record)| {
            record_key.as_str() == update.storage_key
                || (!record.storage_key.is_empty() && record.storage_key == update.storage_key)
        })
        .map(|(_, record)| record_source_kind(record))
}

pub fn merge_discover_kind(
    old: Vec<DiscoverableSkill>,
    kind: SourceKind,
    fresh: Vec<DiscoverableSkill>,
) -> Vec<DiscoverableSkill> {
    let mut merged = old
        .into_iter()
        .filter(|skill| discover_source_kind(skill) != kind)
        .collect::<Vec<_>>();
    merged.extend(fresh);
    deduplicate_discoverable_skills(merged)
}

pub fn merge_update_kind(
    config: &AppConfig,
    old: Vec<SkillUpdateInfo>,
    kind: SourceKind,
    fresh: Vec<SkillUpdateInfo>,
) -> Vec<SkillUpdateInfo> {
    let mut merged = old
        .into_iter()
        .filter(|update| update_source_kind(config, update) != Some(kind))
        .collect::<Vec<_>>();
    merged.extend(fresh);
    let mut seen = HashSet::new();
    merged.retain(|update| seen.insert(update.storage_key.clone()));
    merged
}

pub fn refresh_with_hooks<F>(config: &mut AppConfig, mut refresh: F) -> StartupSkillRefreshResult
where
    F: FnMut(SourceKind) -> Result<FreshSourceResult, String>,
{
    let mut warnings = Vec::new();

    for kind in enabled_source_kinds(config) {
        match refresh(kind) {
            Ok(fresh) => {
                config.skill_discover_cache.skills = merge_discover_kind(
                    std::mem::take(&mut config.skill_discover_cache.skills),
                    kind,
                    fresh.discover_skills,
                );
                let old_updates = std::mem::take(&mut config.skill_update_cache.updates);
                config.skill_update_cache.updates =
                    merge_update_kind(config, old_updates, kind, fresh.pending_updates);
            }
            Err(warning) => warnings.push(warning),
        }
    }

    StartupSkillRefreshResult {
        discover_skills: config.skill_discover_cache.skills.clone(),
        pending_updates: config.skill_update_cache.updates.clone(),
        warnings,
    }
}

pub fn refresh_enabled_sources(
    config: &mut AppConfig,
    main_dir: Option<&Path>,
    app_data_dir: &Path,
) -> StartupSkillRefreshResult {
    let base_config = config.clone();
    let main_dir = main_dir.map(Path::to_path_buf);
    let result = refresh_with_hooks(config, |kind| {
        let mut source_config = base_config.clone();
        let discover_skills = match kind {
            SourceKind::Github => crate::skill_discover::discover_repos_strict(
                &source_config,
                main_dir.as_deref(),
                app_data_dir,
                "github",
            ),
            SourceKind::Gitlab => crate::skill_discover::discover_repos_strict(
                &source_config,
                main_dir.as_deref(),
                app_data_dir,
                "gitlab",
            ),
            SourceKind::SkillHub => crate::skill_hub_discover::discover_all_strict(&source_config),
            SourceKind::IflytekSkillHub => {
                crate::iflytek_skill_hub_discover::discover_all_strict(&source_config)
            }
        }
        .map_err(|err| source_warning(kind, &err.to_dto().message))?;

        let pending_updates = match (kind, main_dir.as_deref()) {
            (_, None) => Ok(Vec::new()),
            (SourceKind::Github, Some(main_dir)) => crate::skill_updates::check_repo_updates_strict(
                &source_config,
                main_dir,
                "github",
            ),
            (SourceKind::Gitlab, Some(main_dir)) => crate::skill_updates::check_repo_updates_strict(
                &source_config,
                main_dir,
                "gitlab",
            ),
            (SourceKind::SkillHub, Some(main_dir)) => {
                crate::skill_updates::check_hub_updates_strict(&mut source_config, main_dir)
            }
            (SourceKind::IflytekSkillHub, Some(_)) => Ok(Vec::new()),
        }
        .map_err(|err| source_warning(kind, &err.to_dto().message))?;

        Ok(FreshSourceResult {
            discover_skills,
            pending_updates,
        })
    });

    if result.warnings.len() < enabled_source_kinds(config).len() {
        let checked_at = crate::skill_discover::iso8601_timestamp_now();
        config.skill_discover_cache.fetched_at = Some(checked_at.clone());
        config.skill_update_cache.checked_at = Some(checked_at);
    }
    StartupSkillRefreshResult {
        discover_skills: config.skill_discover_cache.skills.clone(),
        pending_updates: config.skill_update_cache.updates.clone(),
        warnings: result.warnings,
    }
}

fn source_warning(kind: SourceKind, message: &str) -> String {
    let label = match kind {
        SourceKind::Github => "GitHub",
        SourceKind::Gitlab => "GitLab",
        SourceKind::SkillHub => "Skill Hub",
        SourceKind::IflytekSkillHub => "iFlytek Skill Hub",
    };
    format!("{label} 启动刷新失败：{message}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppConfig, DiscoverableSkill, SkillRecord, SkillUpdateInfo};

    fn discoverable(key: &str, source: &str, host: &str) -> DiscoverableSkill {
        DiscoverableSkill {
            key: key.to_string(),
            source: source.to_string(),
            repo_host: host.to_string(),
            storage_key: key.to_string(),
            ..DiscoverableSkill::default()
        }
    }

    fn update(storage_key: &str, name: &str) -> SkillUpdateInfo {
        SkillUpdateInfo {
            dir_name: name.to_string(),
            name: name.to_string(),
            current_hash: Some("old".to_string()),
            remote_hash: "new".to_string(),
            storage_key: storage_key.to_string(),
        }
    }

    fn config_with_source_records() -> AppConfig {
        let mut config = AppConfig::default();
        config.skill_records.insert(
            "github-key".to_string(),
            SkillRecord {
                storage_key: "github-key".to_string(),
                source: "github".to_string(),
                repo_host: "github.com".to_string(),
                ..SkillRecord::default()
            },
        );
        config.skill_records.insert(
            "gitlab-key".to_string(),
            SkillRecord {
                storage_key: "gitlab-key".to_string(),
                source: "gitlab".to_string(),
                repo_host: "gitlab.internal".to_string(),
                ..SkillRecord::default()
            },
        );
        config.skill_records.insert(
            "hub-key".to_string(),
            SkillRecord {
                storage_key: "hub-key".to_string(),
                source: "skillhub".to_string(),
                ..SkillRecord::default()
            },
        );
        config.skill_records.insert(
            "iflytek-key".to_string(),
            SkillRecord {
                storage_key: "iflytek-key".to_string(),
                source: "iflytek".to_string(),
                ..SkillRecord::default()
            },
        );
        config
    }

    fn config_with_cached_sources() -> AppConfig {
        let mut config = config_with_source_records();
        config.skill_discover_cache.skills = vec![
            discoverable("github-old", "github", "github.com"),
            discoverable("gitlab-old", "gitlab", "gitlab.internal"),
            discoverable("hub-old", "skillhub", ""),
        ];
        config.skill_update_cache.updates = vec![
            update("github-key", "github-old"),
            update("gitlab-key", "gitlab-old"),
            update("hub-key", "hub-old"),
        ];
        config
    }

    fn fresh_source(kind: SourceKind) -> FreshSourceResult {
        match kind {
            SourceKind::Github => FreshSourceResult {
                discover_skills: vec![discoverable("github-new", "github", "github.com")],
                pending_updates: vec![update("github-key", "github-new")],
            },
            SourceKind::Gitlab => FreshSourceResult {
                discover_skills: vec![discoverable(
                    "gitlab-new",
                    "gitlab",
                    "gitlab.internal",
                )],
                pending_updates: vec![update("gitlab-key", "gitlab-new")],
            },
            SourceKind::SkillHub => FreshSourceResult {
                discover_skills: vec![discoverable("hub-new", "skillhub", "")],
                pending_updates: vec![update("hub-key", "hub-new")],
            },
            SourceKind::IflytekSkillHub => FreshSourceResult {
                discover_skills: vec![discoverable("iflytek-new", "iflytek", "")],
                pending_updates: vec![update("iflytek-key", "iflytek-new")],
            },
        }
    }

    #[test]
    fn discover_source_kind_uses_provider_and_skillhub_marker() {
        assert_eq!(
            discover_source_kind(&discoverable("github", "github", "github.com")),
            SourceKind::Github
        );
        assert_eq!(
            discover_source_kind(&discoverable("gitlab", "gitlab", "gitlab.internal")),
            SourceKind::Gitlab
        );
        assert_eq!(
            discover_source_kind(&discoverable("hub", "skillhub", "")),
            SourceKind::SkillHub
        );
        assert_eq!(
            discover_source_kind(&discoverable("iflytek", "iflytek", "")),
            SourceKind::IflytekSkillHub
        );
    }

    #[test]
    fn update_source_kind_uses_storage_key_record() {
        let config = config_with_source_records();
        assert_eq!(
            update_source_kind(&config, &update("github-key", "github")),
            Some(SourceKind::Github)
        );
        assert_eq!(
            update_source_kind(&config, &update("gitlab-key", "gitlab")),
            Some(SourceKind::Gitlab)
        );
        assert_eq!(
            update_source_kind(&config, &update("hub-key", "hub")),
            Some(SourceKind::SkillHub)
        );
        assert_eq!(
            update_source_kind(&config, &update("iflytek-key", "iflytek")),
            Some(SourceKind::IflytekSkillHub)
        );
    }

    #[test]
    fn merge_discover_kind_replaces_only_selected_kind() {
        let old = vec![
            discoverable("github-old", "github", "github.com"),
            discoverable("gitlab-old", "gitlab", "gitlab.internal"),
            discoverable("hub-old", "skillhub", ""),
        ];
        let merged = merge_discover_kind(
            old,
            SourceKind::Gitlab,
            vec![discoverable("gitlab-new", "gitlab", "gitlab.internal")],
        );
        let keys = merged.into_iter().map(|skill| skill.key).collect::<Vec<_>>();
        assert_eq!(keys, vec!["github-old", "hub-old", "gitlab-new"]);
    }

    #[test]
    fn merge_discover_kind_iflytek_preserves_skillhub_entries() {
        let old = vec![
            discoverable("hub-old", "skillhub", ""),
            discoverable("iflytek-old", "iflytek", ""),
        ];
        let merged = merge_discover_kind(
            old,
            SourceKind::IflytekSkillHub,
            vec![discoverable("iflytek-new", "iflytek", "")],
        );
        let keys = merged.into_iter().map(|skill| skill.key).collect::<Vec<_>>();
        assert_eq!(keys, vec!["hub-old", "iflytek-new"]);
    }

    #[test]
    fn merge_update_kind_replaces_only_selected_kind_and_keeps_unknown() {
        let config = config_with_source_records();
        let old = vec![
            update("github-key", "github-old"),
            update("gitlab-key", "gitlab-old"),
            update("hub-key", "hub-old"),
            update("legacy-key", "legacy-old"),
        ];
        let merged = merge_update_kind(
            &config,
            old,
            SourceKind::SkillHub,
            vec![update("hub-key", "hub-new")],
        );
        let names = merged.into_iter().map(|item| item.name).collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["github-old", "gitlab-old", "legacy-old", "hub-new"]
        );
    }

    #[test]
    fn refresh_enabled_kinds_skips_github_by_default() {
        let mut config = config_with_cached_sources();
        let mut calls = Vec::new();

        let result = refresh_with_hooks(&mut config, |kind| {
            calls.push(kind);
            Ok(fresh_source(kind))
        });

        assert_eq!(calls, vec![SourceKind::Gitlab, SourceKind::SkillHub]);
        assert!(result.warnings.is_empty());
        assert_eq!(
            config
                .skill_discover_cache
                .skills
                .iter()
                .map(|skill| skill.key.as_str())
                .collect::<Vec<_>>(),
            vec!["github-old", "gitlab-new", "hub-new"]
        );
    }

    #[test]
    fn failed_kind_keeps_old_discover_and_update_caches() {
        let mut config = config_with_cached_sources();
        let before = config.clone();

        let result = refresh_with_hooks(&mut config, |kind| {
            if kind == SourceKind::Gitlab {
                Err("gitlab unavailable".to_string())
            } else {
                Ok(fresh_source(kind))
            }
        });

        let old_gitlab_discover = before
            .skill_discover_cache
            .skills
            .iter()
            .find(|skill| discover_source_kind(skill) == SourceKind::Gitlab);
        let new_gitlab_discover = config
            .skill_discover_cache
            .skills
            .iter()
            .find(|skill| discover_source_kind(skill) == SourceKind::Gitlab);
        assert_eq!(new_gitlab_discover, old_gitlab_discover);

        let old_gitlab_update = before
            .skill_update_cache
            .updates
            .iter()
            .find(|update| update_source_kind(&before, update) == Some(SourceKind::Gitlab));
        let new_gitlab_update = config
            .skill_update_cache
            .updates
            .iter()
            .find(|update| update_source_kind(&config, update) == Some(SourceKind::Gitlab));
        assert_eq!(new_gitlab_update, old_gitlab_update);
        assert_eq!(result.warnings, vec!["gitlab unavailable"]);
    }

    #[test]
    fn refresh_enabled_kinds_includes_iflytek_when_enabled() {
        let mut config = config_with_cached_sources();
        config.settings.startup_refresh.iflytek_skill_hub = true;
        config.skill_discover_cache.skills.push(discoverable("iflytek-old", "iflytek", ""));
        let mut calls = Vec::new();

        let result = refresh_with_hooks(&mut config, |kind| {
            calls.push(kind);
            Ok(fresh_source(kind))
        });

        assert_eq!(
            calls,
            vec![
                SourceKind::Gitlab,
                SourceKind::SkillHub,
                SourceKind::IflytekSkillHub,
            ]
        );
        assert!(result.warnings.is_empty());
        assert_eq!(
            config
                .skill_discover_cache
                .skills
                .iter()
                .map(|skill| skill.key.as_str())
                .collect::<Vec<_>>(),
            vec!["github-old", "gitlab-new", "hub-new", "iflytek-new"]
        );
    }
}
