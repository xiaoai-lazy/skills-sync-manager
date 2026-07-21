use crate::models::{AppConfig, AppError, DiscoverableSkill, IflytekSkillHubEndpoint, SkillDiscoverCache};
use crate::skill_discover::{
    deduplicate_discoverable_skills, filter_uninstalled_discoverable_skills, iso8601_timestamp_now,
};
use crate::iflytek_skill_hub_client::{self, IflytekSkillDto};
use crate::skill_storage;
use std::collections::HashSet;

pub fn iflytek_skill_to_discoverable(
    endpoint_id: &str,
    dto: &IflytekSkillDto,
) -> DiscoverableSkill {
    let directory = format!("{}/{}", dto.namespace, dto.slug);
    let storage_key =
        skill_storage::storage_key_for_hub(endpoint_id, &dto.namespace, &dto.slug);

    DiscoverableSkill {
        key: format!("{}:{}/{}", endpoint_id, dto.namespace, dto.slug),
        name: dto.name.clone(),
        description: dto.description.clone(),
        directory,
        install_dir_name: dto.slug.clone(),
        repo_host: String::new(),
        project_path: String::new(),
        repo_owner: String::new(),
        repo_name: String::new(),
        repo_branch: String::new(),
        source: "iflytek".to_string(),
        storage_key,
        link_name: dto.slug.clone(),
        repo_slug: String::new(),
        hub_endpoint_id: endpoint_id.to_string(),
        hub_skill_group: dto.namespace.clone(),
        hub_skill_id: dto.slug.clone(),
    }
}

pub fn discover_iflytek_endpoint(
    endpoint: &IflytekSkillHubEndpoint,
) -> Result<Vec<DiscoverableSkill>, AppError> {
    let dtos = iflytek_skill_hub_client::fetch_skills(&endpoint.base_url)?;
    Ok(dtos
        .iter()
        .map(|dto| iflytek_skill_to_discoverable(&endpoint.id, dto))
        .collect())
}

/// 扫描所有已启用 iFlytek 端点；单个端点失败时跳过并记录警告，不中断其余端点。
pub fn discover_all(config: &AppConfig) -> (Vec<DiscoverableSkill>, Vec<String>) {
    let mut skills = Vec::new();
    let mut warnings = Vec::new();

    for endpoint in config
        .iflytek_skill_hub_endpoints
        .iter()
        .filter(|endpoint| endpoint.enabled)
    {
        match discover_iflytek_endpoint(endpoint) {
            Ok(endpoint_skills) => skills.extend(endpoint_skills),
            Err(err) => {
                warnings.push(format!(
                    "跳过来源 {}：{}",
                    iflytek_endpoint_display_label(endpoint),
                    err.to_dto().message
                ));
            }
        }
    }

    (skills, warnings)
}

pub fn discover_all_strict(config: &AppConfig) -> Result<Vec<DiscoverableSkill>, AppError> {
    let mut skills = Vec::new();
    for endpoint in config
        .iflytek_skill_hub_endpoints
        .iter()
        .filter(|endpoint| endpoint.enabled)
    {
        skills.extend(discover_iflytek_endpoint(endpoint)?);
    }

    let filtered = filter_uninstalled_discoverable_skills(
        skills,
        config.settings.main_skills_dir.as_deref(),
        Some(&config.skill_records),
    );
    Ok(deduplicate_discoverable_skills(filtered))
}

fn iflytek_endpoint_display_label(endpoint: &IflytekSkillHubEndpoint) -> String {
    if endpoint.name.is_empty() {
        endpoint.base_url.clone()
    } else {
        format!("{} ({})", endpoint.name, endpoint.base_url)
    }
}

fn skill_belongs_to_iflytek(skill: &DiscoverableSkill, iflytek_endpoint_id: &str) -> bool {
    skill.source == "iflytek" && skill.hub_endpoint_id == iflytek_endpoint_id
}

pub fn merge_iflytek_into_discover_cache(
    config: &mut AppConfig,
    skills: Vec<DiscoverableSkill>,
) -> Vec<DiscoverableSkill> {
    let iflytek_endpoint_ids: HashSet<String> = skills
        .iter()
        .map(|skill| skill.hub_endpoint_id.clone())
        .filter(|id| !id.is_empty())
        .collect();

    let retained = config
        .skill_discover_cache
        .skills
        .iter()
        .filter(|skill| {
            !(skill.source == "iflytek"
                && iflytek_endpoint_ids.contains(&skill.hub_endpoint_id))
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

pub fn remove_iflytek_from_discover_cache(
    config: &mut AppConfig,
    iflytek_endpoint_id: &str,
) -> Vec<DiscoverableSkill> {
    let skills = config
        .skill_discover_cache
        .skills
        .iter()
        .filter(|skill| !skill_belongs_to_iflytek(skill, iflytek_endpoint_id))
        .cloned()
        .collect::<Vec<_>>();

    config.skill_discover_cache = SkillDiscoverCache {
        fetched_at: Some(iso8601_timestamp_now()),
        skills: skills.clone(),
    };

    skills
}

pub fn merge_iflytek_endpoint_into_discover_cache(
    config: &mut AppConfig,
    iflytek_endpoint_id: &str,
) -> Result<(), AppError> {
    let endpoint = config
        .iflytek_skill_hub_endpoints
        .iter()
        .find(|endpoint| endpoint.id == iflytek_endpoint_id)
        .ok_or_else(|| AppError::Io {
            path: None,
            message: format!("iFlytek Hub 端点不存在: {iflytek_endpoint_id}"),
        })?
        .clone();

    if !endpoint.enabled {
        remove_iflytek_from_discover_cache(config, iflytek_endpoint_id);
        return Ok(());
    }

    let iflytek_skills = discover_iflytek_endpoint(&endpoint)?;
    let main_dir = config.settings.main_skills_dir.as_deref();
    let filtered = filter_uninstalled_discoverable_skills(
        iflytek_skills,
        main_dir,
        Some(&config.skill_records),
    );
    merge_iflytek_into_discover_cache(config, filtered);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iflytek_skill_hub_client::IflytekSkillDto;
    use crate::skill_hub_discover::hub_skill_to_discoverable;
    use crate::skill_hub_client::HubSkillDto;

    fn sample_dto() -> IflytekSkillDto {
        IflytekSkillDto {
            slug: "tdd".to_string(),
            name: "tdd".to_string(),
            description: "d".to_string(),
            namespace: "global".to_string(),
            latest_version: Some("1".to_string()),
        }
    }

    #[test]
    fn iflytek_skill_to_discoverable_sets_source_and_storage_key() {
        let dto = sample_dto();
        let skill = iflytek_skill_to_discoverable("xkw", &dto);

        assert_eq!(skill.source, "iflytek");
        assert_eq!(skill.storage_key, "hub/xkw/global/tdd");
        assert_eq!(skill.hub_skill_group, "global");
        assert_eq!(skill.hub_skill_id, "tdd");
        assert_eq!(skill.key, "xkw:global/tdd");
        assert_eq!(skill.directory, "global/tdd");
        assert_eq!(skill.install_dir_name, "tdd");
        assert_eq!(skill.link_name, "tdd");
        assert_eq!(skill.hub_endpoint_id, "xkw");
        assert_eq!(skill.name, "tdd");
        assert_eq!(skill.description, "d");
    }

    #[test]
    fn merge_iflytek_does_not_clear_skills_sync_cache() {
        let skills_sync_skill = hub_skill_to_discoverable(
            "company-hub",
            &HubSkillDto {
                id: "brainstorming".to_string(),
                group: "common".to_string(),
                name: "Brainstorming".to_string(),
                description: "Keep me.".to_string(),
                hash: None,
            },
        );
        let stale_iflytek_skill = iflytek_skill_to_discoverable(
            "xkw",
            &IflytekSkillDto {
                slug: "old-skill".to_string(),
                name: "Old".to_string(),
                description: "Stale.".to_string(),
                namespace: "global".to_string(),
                latest_version: None,
            },
        );
        let fresh_iflytek_skill = iflytek_skill_to_discoverable("xkw", &sample_dto());

        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: None,
                skills: vec![skills_sync_skill.clone(), stale_iflytek_skill],
            },
            ..Default::default()
        };

        let merged = merge_iflytek_into_discover_cache(&mut config, vec![fresh_iflytek_skill.clone()]);

        assert_eq!(merged.len(), 2);
        assert!(merged.iter().any(|skill| skill.key == skills_sync_skill.key));
        assert!(merged.iter().any(|skill| skill.key == fresh_iflytek_skill.key));
        assert_eq!(config.skill_discover_cache.skills, merged);
    }

    #[test]
    fn remove_iflytek_from_discover_cache_keeps_other_sources() {
        let skills_sync_skill = hub_skill_to_discoverable(
            "company-hub",
            &HubSkillDto {
                id: "tdd".to_string(),
                group: "common".to_string(),
                name: "TDD".to_string(),
                description: "Keep me.".to_string(),
                hash: None,
            },
        );
        let iflytek_skill = iflytek_skill_to_discoverable("xkw", &sample_dto());

        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: None,
                skills: vec![skills_sync_skill.clone(), iflytek_skill],
            },
            ..Default::default()
        };

        let remaining = remove_iflytek_from_discover_cache(&mut config, "xkw");

        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].key, skills_sync_skill.key);
        assert_eq!(config.skill_discover_cache.skills, remaining);
    }
}
