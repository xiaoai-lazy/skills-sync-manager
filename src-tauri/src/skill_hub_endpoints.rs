use crate::models::{AppConfig, AppError, SkillHubEndpoint};
use crate::skill_hub_client;
use crate::skill_hub_discover;

fn find_endpoint_index(config: &AppConfig, id: &str) -> Option<usize> {
    config
        .skill_hub_endpoints
        .iter()
        .position(|endpoint| endpoint.id == id)
}

fn find_endpoint_index_by_base_url(config: &AppConfig, base_url: &str) -> Option<usize> {
    config
        .skill_hub_endpoints
        .iter()
        .position(|endpoint| endpoint.base_url == base_url)
}

fn endpoint_not_found(id: &str) -> AppError {
    AppError::InvalidInput {
        input: id.to_string(),
        message: format!("Hub 端点不存在: {id}"),
    }
}

fn normalize_and_validate_base_url(base_url: &str) -> Result<String, AppError> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput {
            input: base_url.to_string(),
            message: "Hub base URL 不能为空".to_string(),
        });
    }

    let without_slash = skill_hub_client::normalize_base_url(trimmed);
    let for_parse = if without_slash.contains("://") {
        without_slash.clone()
    } else {
        format!("https://{without_slash}")
    };

    reqwest::Url::parse(&for_parse).map_err(|_| AppError::InvalidInput {
        input: base_url.to_string(),
        message: "Hub base URL 格式无效".to_string(),
    })?;

    Ok(if without_slash.contains("://") {
        without_slash
    } else {
        for_parse
    })
}

fn validate_endpoint_id(id: &str) -> Result<(), AppError> {
    if id.trim().is_empty() {
        return Err(AppError::InvalidInput {
            input: id.to_string(),
            message: "Hub 端点 ID 不能为空".to_string(),
        });
    }
    Ok(())
}

fn slugify_part(part: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in part.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };
        if mapped == '-' {
            if last_dash {
                continue;
            }
            last_dash = true;
        } else {
            last_dash = false;
        }
        slug.push(mapped);
    }
    slug.trim_matches('-').to_string()
}

fn generate_endpoint_id_from_url(base_url: &str, config: &AppConfig) -> Result<String, AppError> {
    let for_parse = if base_url.contains("://") {
        base_url.to_string()
    } else {
        format!("https://{base_url}")
    };
    let url = reqwest::Url::parse(&for_parse).map_err(|_| AppError::InvalidInput {
        input: base_url.to_string(),
        message: "Hub base URL 格式无效".to_string(),
    })?;

    let mut parts = Vec::new();
    if let Some(host) = url.host_str() {
        let host_slug = slugify_part(host);
        if !host_slug.is_empty() {
            parts.push(host_slug);
        }
    }
    if let Some(port) = url.port() {
        parts.push(port.to_string());
    }
    let path = url.path().trim_matches('/');
    if !path.is_empty() {
        let path_slug = slugify_part(path.replace('/', "-").as_str());
        if !path_slug.is_empty() {
            parts.push(path_slug);
        }
    }

    let base_id = if parts.is_empty() {
        "skill-hub".to_string()
    } else {
        parts.join("-")
    };

    Ok(ensure_unique_endpoint_id(config, &base_id))
}

fn ensure_unique_endpoint_id(config: &AppConfig, base_id: &str) -> String {
    if find_endpoint_index(config, base_id).is_none() {
        return base_id.to_string();
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base_id}-{suffix}");
        if find_endpoint_index(config, &candidate).is_none() {
            return candidate;
        }
        suffix += 1;
    }
}

pub fn list_skill_hub_endpoints(config: &AppConfig) -> Vec<SkillHubEndpoint> {
    config.skill_hub_endpoints.clone()
}

pub fn add_skill_hub_endpoint(
    config: &mut AppConfig,
    name: &str,
    base_url: &str,
) -> Result<(), AppError> {
    let base_url = normalize_and_validate_base_url(base_url)?;

    if find_endpoint_index_by_base_url(config, &base_url).is_some() {
        return Ok(());
    }

    let id = generate_endpoint_id_from_url(&base_url, config)?;
    config.skill_hub_endpoints.push(SkillHubEndpoint {
        id: id.clone(),
        name: name.trim().to_string(),
        base_url,
        enabled: true,
    });

    skill_hub_discover::merge_hub_endpoint_into_discover_cache(config, &id)
}

pub fn remove_skill_hub_endpoint(config: &mut AppConfig, id: &str) -> Result<(), AppError> {
    validate_endpoint_id(id)?;
    let index = find_endpoint_index(config, id).ok_or_else(|| endpoint_not_found(id))?;
    config.skill_hub_endpoints.remove(index);
    skill_hub_discover::remove_hub_from_discover_cache(config, id);
    Ok(())
}

pub fn set_skill_hub_endpoint_enabled(
    config: &mut AppConfig,
    id: &str,
    enabled: bool,
) -> Result<(), AppError> {
    validate_endpoint_id(id)?;
    let index = find_endpoint_index(config, id).ok_or_else(|| endpoint_not_found(id))?;
    config.skill_hub_endpoints[index].enabled = enabled;

    if enabled {
        skill_hub_discover::merge_hub_endpoint_into_discover_cache(config, id)
    } else {
        skill_hub_discover::remove_hub_from_discover_cache(config, id);
        Ok(())
    }
}

pub fn hub_endpoint_base_url(config: &AppConfig, id: &str) -> Result<String, AppError> {
    validate_endpoint_id(id)?;
    config
        .skill_hub_endpoints
        .iter()
        .find(|endpoint| endpoint.id == id)
        .map(|endpoint| endpoint.base_url.clone())
        .ok_or_else(|| endpoint_not_found(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DiscoverableSkill, SkillDiscoverCache};
    use crate::skill_hub_discover::hub_skill_to_discoverable;
    use crate::skill_hub_client::HubSkillDto;

    fn sample_hub_skill(endpoint_id: &str) -> DiscoverableSkill {
        hub_skill_to_discoverable(
            endpoint_id,
            &HubSkillDto {
                id: "tdd".to_string(),
                group: "common".to_string(),
                name: "TDD".to_string(),
                description: "Test-driven development.".to_string(),
                hash: None,
            },
        )
    }

    fn config_with_endpoint(endpoint: SkillHubEndpoint) -> AppConfig {
        AppConfig {
            skill_hub_endpoints: vec![endpoint],
            ..Default::default()
        }
    }

    #[test]
    fn generate_endpoint_id_from_host_and_port() {
        let config = AppConfig::default();
        let id = generate_endpoint_id_from_url("http://localhost:3337", &config).expect("id");
        assert_eq!(id, "localhost-3337");
    }

    #[test]
    fn generate_endpoint_id_from_hostname() {
        let config = AppConfig::default();
        let id =
            generate_endpoint_id_from_url("https://hub.example.com/", &config).expect("id");
        assert_eq!(id, "hub-example-com");
    }

    #[test]
    fn generate_endpoint_id_avoids_collision() {
        let config = config_with_endpoint(SkillHubEndpoint {
            id: "hub-example-com".to_string(),
            name: "Existing".to_string(),
            base_url: "https://other.example.com".to_string(),
            enabled: true,
        });
        let id =
            generate_endpoint_id_from_url("https://hub.example.com", &config).expect("id");
        assert_eq!(id, "hub-example-com-2");
    }

    #[test]
    fn add_skill_hub_endpoint_dedupes_by_url() {
        let mut config = config_with_endpoint(SkillHubEndpoint {
            id: "company-hub".to_string(),
            name: "Company Hub".to_string(),
            base_url: "https://hub.example.com".to_string(),
            enabled: true,
        });

        add_skill_hub_endpoint(&mut config, "Duplicate Name", "https://hub.example.com/")
            .expect("duplicate add should succeed");

        assert_eq!(config.skill_hub_endpoints.len(), 1);
        assert_eq!(config.skill_hub_endpoints[0].name, "Company Hub");
        assert_eq!(
            config.skill_hub_endpoints[0].base_url,
            "https://hub.example.com"
        );
    }

    #[test]
    fn add_skill_hub_endpoint_generates_id_and_normalizes_trailing_slash() {
        let mut config = AppConfig::default();

        let result = add_skill_hub_endpoint(
            &mut config,
            "Company Hub",
            "https://hub.example.com/",
        );

        assert!(result.is_err(), "network fetch should fail in unit test");
        assert_eq!(config.skill_hub_endpoints.len(), 1);
        assert_eq!(config.skill_hub_endpoints[0].id, "hub-example-com");
        assert_eq!(config.skill_hub_endpoints[0].name, "Company Hub");
        assert_eq!(
            config.skill_hub_endpoints[0].base_url,
            "https://hub.example.com"
        );
        assert!(config.skill_hub_endpoints[0].enabled);
    }

    #[test]
    fn disable_hub_endpoint_removes_cached_skills() {
        let endpoint = SkillHubEndpoint {
            id: "company-hub".to_string(),
            name: "Company Hub".to_string(),
            base_url: "https://hub.example.com".to_string(),
            enabled: true,
        };
        let hub_skill = sample_hub_skill("company-hub");
        let github_skill = DiscoverableSkill {
            key: "github.com/obra/superpowers:skills/brainstorming".to_string(),
            name: "brainstorming".to_string(),
            description: "Keep me.".to_string(),
            directory: "skills/brainstorming".to_string(),
            install_dir_name: "brainstorming".to_string(),
            source: "github".to_string(),
            ..Default::default()
        };

        let mut config = AppConfig {
            skill_hub_endpoints: vec![endpoint],
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: None,
                skills: vec![github_skill.clone(), hub_skill],
            },
            ..Default::default()
        };

        set_skill_hub_endpoint_enabled(&mut config, "company-hub", false)
            .expect("disable endpoint");

        assert!(!config.skill_hub_endpoints[0].enabled);
        assert_eq!(config.skill_discover_cache.skills.len(), 1);
        assert_eq!(config.skill_discover_cache.skills[0].key, github_skill.key);
    }

    #[test]
    fn remove_hub_endpoint_removes_cached_skills() {
        let endpoint = SkillHubEndpoint {
            id: "company-hub".to_string(),
            name: "Company Hub".to_string(),
            base_url: "https://hub.example.com".to_string(),
            enabled: true,
        };
        let hub_skill = sample_hub_skill("company-hub");

        let mut config = AppConfig {
            skill_hub_endpoints: vec![endpoint],
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: None,
                skills: vec![hub_skill],
            },
            ..Default::default()
        };

        remove_skill_hub_endpoint(&mut config, "company-hub").expect("remove endpoint");

        assert!(config.skill_hub_endpoints.is_empty());
        assert!(config.skill_discover_cache.skills.is_empty());
    }

    #[test]
    fn add_rejects_empty_base_url_and_invalid_url() {
        let mut config = AppConfig::default();

        let url_error = add_skill_hub_endpoint(&mut config, "Hub", "")
            .expect_err("empty url");
        assert!(matches!(url_error, AppError::InvalidInput { .. }));

        let url_error =
            add_skill_hub_endpoint(&mut config, "Hub", "not a url").expect_err("bad url");
        assert!(matches!(url_error, AppError::InvalidInput { .. }));
        assert!(config.skill_hub_endpoints.is_empty());
    }
}
