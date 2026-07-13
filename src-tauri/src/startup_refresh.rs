use crate::models::{
    default_github_host, AppConfig, DiscoverableSkill, SkillRecord, SkillUpdateInfo,
};
use crate::skill_discover::deduplicate_discoverable_skills;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Github,
    Gitlab,
    SkillHub,
}

fn record_source_kind(record: &SkillRecord) -> SourceKind {
    if record.source == "skillhub" {
        SourceKind::SkillHub
    } else if record.source == "gitlab" || record.repo_host != default_github_host() {
        SourceKind::Gitlab
    } else {
        SourceKind::Github
    }
}

pub fn discover_source_kind(skill: &DiscoverableSkill) -> SourceKind {
    if skill.source == "skillhub" {
        SourceKind::SkillHub
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
        config
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
}
