use crate::models::{AppConfig, AppError, DiscoverableSkill, SkillDiscoverCache, SkillHubEndpoint};
use crate::skill_discover::{
    deduplicate_discoverable_skills, filter_uninstalled_discoverable_skills, iso8601_timestamp_now,
};
use crate::skill_hub_client::{self, HubSkillDto};
use crate::skill_storage;
use std::collections::HashSet;

pub fn hub_skill_to_discoverable(hub_endpoint_id: &str, dto: &HubSkillDto) -> DiscoverableSkill {
    let directory = format!("{}/{}", dto.group, dto.id);
    let storage_key =
        skill_storage::storage_key_for_hub(hub_endpoint_id, &dto.group, &dto.id);

    DiscoverableSkill {
        key: format!("{}:{}/{}", hub_endpoint_id, dto.group, dto.id),
        name: dto.name.clone(),
        description: dto.description.clone(),
        directory,
        install_dir_name: dto.id.clone(),
        repo_host: String::new(),
        project_path: String::new(),
        repo_owner: String::new(),
        repo_name: String::new(),
        repo_branch: String::new(),
        source: "skillhub".to_string(),
        storage_key,
        link_name: dto.id.clone(),
        repo_slug: String::new(),
        hub_endpoint_id: hub_endpoint_id.to_string(),
        hub_skill_group: dto.group.clone(),
        hub_skill_id: dto.id.clone(),
    }
}

pub fn discover_hub_endpoint(
    endpoint: &SkillHubEndpoint,
) -> Result<Vec<DiscoverableSkill>, AppError> {
    let dtos = skill_hub_client::fetch_skills(&endpoint.base_url, None)?;
    Ok(dtos
        .iter()
        .map(|dto| hub_skill_to_discoverable(&endpoint.id, dto))
        .collect())
}

/// 扫描所有已启用 Hub 端点；单个端点失败时跳过并记录警告，不中断其余端点。
pub fn discover_all(config: &AppConfig) -> (Vec<DiscoverableSkill>, Vec<String>) {
    let mut skills = Vec::new();
    let mut warnings = Vec::new();

    for endpoint in config
        .skill_hub_endpoints
        .iter()
        .filter(|endpoint| endpoint.enabled)
    {
        match discover_hub_endpoint(endpoint) {
            Ok(endpoint_skills) => skills.extend(endpoint_skills),
            Err(err) => {
                warnings.push(format!(
                    "跳过来源 {}：{}",
                    hub_endpoint_display_label(endpoint),
                    err.to_dto().message
                ));
            }
        }
    }

    (skills, warnings)
}

fn hub_endpoint_display_label(endpoint: &SkillHubEndpoint) -> String {
    if endpoint.name.is_empty() {
        endpoint.base_url.clone()
    } else {
        format!("{} ({})", endpoint.name, endpoint.base_url)
    }
}

fn skill_belongs_to_hub(skill: &DiscoverableSkill, hub_endpoint_id: &str) -> bool {
    skill.source == "skillhub" && skill.hub_endpoint_id == hub_endpoint_id
}

pub fn merge_hub_into_discover_cache(
    config: &mut AppConfig,
    skills: Vec<DiscoverableSkill>,
) -> Vec<DiscoverableSkill> {
    let hub_endpoint_ids: HashSet<String> = skills
        .iter()
        .map(|skill| skill.hub_endpoint_id.clone())
        .filter(|id| !id.is_empty())
        .collect();

    let retained = config
        .skill_discover_cache
        .skills
        .iter()
        .filter(|skill| {
            !(skill.source == "skillhub"
                && hub_endpoint_ids.contains(&skill.hub_endpoint_id))
        })
        .cloned()
        .collect::<Vec<_>>();

    let mut merged = retained;
    merged.extend(skills);
    let skills = deduplicate_discoverable_skills(merged);

    config.skill_discover_cache = SkillDiscoverCache {
        fetched_at: Some(iso8601_timestamp_now()),
        skills: skills.clone(),
    };

    skills
}

pub fn remove_hub_from_discover_cache(
    config: &mut AppConfig,
    hub_endpoint_id: &str,
) -> Vec<DiscoverableSkill> {
    let skills = config
        .skill_discover_cache
        .skills
        .iter()
        .filter(|skill| !skill_belongs_to_hub(skill, hub_endpoint_id))
        .cloned()
        .collect::<Vec<_>>();

    config.skill_discover_cache = SkillDiscoverCache {
        fetched_at: Some(iso8601_timestamp_now()),
        skills: skills.clone(),
    };

    skills
}

pub fn merge_hub_endpoint_into_discover_cache(
    config: &mut AppConfig,
    hub_endpoint_id: &str,
) -> Result<(), AppError> {
    let endpoint = config
        .skill_hub_endpoints
        .iter()
        .find(|endpoint| endpoint.id == hub_endpoint_id)
        .ok_or_else(|| AppError::Io {
            path: None,
            message: format!("Hub 端点不存在: {hub_endpoint_id}"),
        })?
        .clone();

    if !endpoint.enabled {
        remove_hub_from_discover_cache(config, hub_endpoint_id);
        return Ok(());
    }

    let hub_skills = discover_hub_endpoint(&endpoint)?;
    let main_dir = config.settings.main_skills_dir.as_deref();
    let filtered = filter_uninstalled_discoverable_skills(
        hub_skills,
        main_dir,
        Some(&config.skill_records),
    );
    merge_hub_into_discover_cache(config, filtered);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_hub_client::HubSkillDto;

    fn sample_dto() -> HubSkillDto {
        HubSkillDto {
            id: "tdd".to_string(),
            group: "common".to_string(),
            name: "TDD".to_string(),
            description: "Test-driven development.".to_string(),
            hash: Some("abc123".to_string()),
        }
    }

    #[test]
    fn hub_skill_to_discoverable_maps_fields_per_spec() {
        let dto = sample_dto();
        let skill = hub_skill_to_discoverable("company-hub", &dto);

        assert_eq!(skill.source, "skillhub");
        assert_eq!(skill.key, "company-hub:common/tdd");
        assert_eq!(
            skill.storage_key,
            "hub/company-hub/common/tdd"
        );
        assert_eq!(skill.link_name, "tdd");
        assert_eq!(skill.hub_endpoint_id, "company-hub");
        assert_eq!(skill.hub_skill_group, "common");
        assert_eq!(skill.hub_skill_id, "tdd");
        assert_eq!(skill.directory, "common/tdd");
        assert_eq!(skill.install_dir_name, "tdd");
        assert_eq!(skill.name, "TDD");
        assert_eq!(skill.description, "Test-driven development.");
        assert!(skill.repo_host.is_empty());
        assert!(skill.project_path.is_empty());
        assert!(skill.repo_owner.is_empty());
        assert!(skill.repo_name.is_empty());
        assert!(skill.repo_branch.is_empty());
        assert!(skill.repo_slug.is_empty());
    }

    #[test]
    fn merge_hub_into_discover_cache_replaces_same_endpoint_entries() {
        let github_skill = DiscoverableSkill {
            key: "github.com/obra/superpowers:skills/brainstorming".to_string(),
            name: "brainstorming".to_string(),
            description: "Git skill.".to_string(),
            directory: "skills/brainstorming".to_string(),
            install_dir_name: "brainstorming".to_string(),
            source: "github".to_string(),
            ..Default::default()
        };
        let stale_hub_skill = hub_skill_to_discoverable(
            "company-hub",
            &HubSkillDto {
                id: "old-skill".to_string(),
                group: "common".to_string(),
                name: "Old".to_string(),
                description: "Stale.".to_string(),
                hash: None,
            },
        );
        let fresh_hub_skill = hub_skill_to_discoverable("company-hub", &sample_dto());

        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: None,
                skills: vec![github_skill.clone(), stale_hub_skill],
            },
            ..Default::default()
        };

        let merged = merge_hub_into_discover_cache(&mut config, vec![fresh_hub_skill.clone()]);

        assert_eq!(merged.len(), 2);
        assert!(merged.iter().any(|skill| skill.key == github_skill.key));
        assert!(merged.iter().any(|skill| skill.key == fresh_hub_skill.key));
        assert_eq!(config.skill_discover_cache.skills, merged);
    }

    #[test]
    fn remove_hub_from_discover_cache_keeps_other_sources() {
        let github_skill = DiscoverableSkill {
            key: "github.com/obra/superpowers:skills/brainstorming".to_string(),
            name: "brainstorming".to_string(),
            description: "Keep me.".to_string(),
            directory: "skills/brainstorming".to_string(),
            install_dir_name: "brainstorming".to_string(),
            source: "github".to_string(),
            ..Default::default()
        };
        let hub_skill = hub_skill_to_discoverable("company-hub", &sample_dto());

        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: None,
                skills: vec![github_skill.clone(), hub_skill],
            },
            ..Default::default()
        };

        let remaining = remove_hub_from_discover_cache(&mut config, "company-hub");

        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].key, github_skill.key);
        assert_eq!(config.skill_discover_cache.skills, remaining);
    }
}
