use crate::credential_store;
use crate::gitlab_client;
use crate::models::{
    AppConfig, AppError, PreviewAddRepoResult, SkillRepo, default_github_host,
    default_github_provider,
};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRepoUrl {
    pub host: String,
    pub provider: String,
    pub project_path: String,
    pub branch: String,
    pub owner: String,
    pub name: String,
}

pub fn canonical_repo_key(host: &str, project_path: &str) -> String {
    format!(
        "{}/{}",
        host.trim().to_lowercase(),
        project_path.trim().to_lowercase()
    )
}

fn find_repo_index(config: &AppConfig, host: &str, project_path: &str) -> Option<usize> {
    let key = canonical_repo_key(host, project_path);
    config.skill_repos.iter().position(|repo| {
        canonical_repo_key(&repo.host, &repo.project_path) == key
    })
}

fn split_owner_name(project_path: &str) -> (String, String) {
    match project_path.rsplit_once('/') {
        Some((owner, name)) => (owner.to_string(), name.to_string()),
        None => (String::new(), project_path.to_string()),
    }
}

fn normalize_host(host: &str) -> String {
    host.trim()
        .trim_end_matches('/')
        .strip_prefix("www.")
        .unwrap_or(host.trim())
        .to_lowercase()
}

fn is_github_host(host: &str) -> bool {
    matches!(normalize_host(host).as_str(), "github.com")
}

/// Removes duplicate repos (same host/project_path, case-insensitive). Returns true when entries were removed.
pub fn dedupe_skill_repos(config: &mut AppConfig) -> bool {
    let before = config.skill_repos.len();
    let mut seen = HashSet::new();
    config.skill_repos.retain(|repo| {
        seen.insert(canonical_repo_key(&repo.host, &repo.project_path))
    });
    config.skill_repos.len() != before
}

pub fn parse_repo_url(url: &str) -> Result<ParsedRepoUrl, AppError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput {
            input: url.to_string(),
            message: "仓库 URL 不能为空".to_string(),
        });
    }

    let (host, path) = extract_host_and_path(trimmed)?;
    let provider = if is_github_host(&host) {
        default_github_provider()
    } else {
        "gitlab".to_string()
    };

    let (project_path, branch) = if provider == "gitlab" {
        extract_gitlab_project_and_branch(&path, trimmed)?
    } else {
        extract_github_project_and_branch(&path, trimmed)?
    };

    let (owner, name) = split_owner_name(&project_path);

    Ok(ParsedRepoUrl {
        host: normalize_host(&host),
        provider,
        project_path,
        branch,
        owner,
        name,
    })
}

fn extract_host_and_path(input: &str) -> Result<(String, String), AppError> {
    let trimmed = input.trim();

    if let Some(after_git) = trimmed.strip_prefix("git@") {
        let (host, path) = after_git.split_once(':').ok_or_else(|| AppError::InvalidInput {
            input: input.to_string(),
            message: "SSH 仓库 URL 格式无效".to_string(),
        })?;
        let path = path
            .strip_suffix(".git")
            .unwrap_or(path)
            .trim_end_matches('/')
            .to_string();
        return Ok((normalize_host(host), path));
    }

    if let Some(after_scheme) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
    {
        let path_only = strip_url_query_and_fragment(after_scheme);
        let (host, path) = path_only.split_once('/').ok_or_else(|| AppError::InvalidInput {
            input: input.to_string(),
            message: "仓库 URL 格式无效，缺少项目路径".to_string(),
        })?;
        return Ok((
            normalize_host(host),
            path.trim_end_matches('/').to_string(),
        ));
    }

    if let Some(path) = trimmed
        .strip_prefix("www.github.com/")
        .or_else(|| trimmed.strip_prefix("github.com/"))
    {
        return Ok((
            default_github_host(),
            path.trim_end_matches('/').to_string(),
        ));
    }

    let path_only = strip_url_query_and_fragment(trimmed);
    if let Some((first, rest)) = path_only.split_once('/') {
        if first.contains('.') {
            return Ok((
                normalize_host(first),
                rest.trim_end_matches('/').to_string(),
            ));
        }
    }

    Ok((
        default_github_host(),
        path_only.trim_end_matches('/').to_string(),
    ))
}

fn extract_github_project_and_branch(
    path: &str,
    original_input: &str,
) -> Result<(String, String), AppError> {
    let path_only = path.strip_suffix(".git").unwrap_or(path);
    let parts: Vec<&str> = path_only.split('/').filter(|part| !part.is_empty()).collect();

    if parts.len() < 2 {
        return Err(AppError::InvalidInput {
            input: original_input.to_string(),
            message: "仓库 URL 格式无效，需要 owner/name".to_string(),
        });
    }

    let owner = parts[0].trim().to_lowercase();
    let name = parts[1]
        .strip_suffix(".git")
        .unwrap_or(parts[1])
        .trim()
        .to_lowercase();

    if owner.is_empty() || name.is_empty() {
        return Err(AppError::InvalidInput {
            input: original_input.to_string(),
            message: "仓库 owner 或 name 不能为空".to_string(),
        });
    }

    let tree_branch = extract_github_tree_branch(&parts);
    let branch = resolve_repo_branch(original_input, tree_branch);

    Ok((format!("{}/{}", owner, name), branch))
}

fn extract_gitlab_project_and_branch(
    path: &str,
    original_input: &str,
) -> Result<(String, String), AppError> {
    let (project_segment, remainder) = if let Some(index) = path.find("/-/") {
        (&path[..index], Some(&path[index..]))
    } else {
        (path, None)
    };

    let project_path = project_segment
        .strip_suffix(".git")
        .unwrap_or(project_segment)
        .trim()
        .trim_end_matches('/')
        .to_lowercase();

    if project_path.is_empty() {
        return Err(AppError::InvalidInput {
            input: original_input.to_string(),
            message: "GitLab 项目路径不能为空".to_string(),
        });
    }

    let branch = remainder
        .and_then(extract_gitlab_tree_branch)
        .or_else(|| parse_branch_from_source_url(original_input))
        .unwrap_or_else(|| "main".to_string());

    Ok((project_path, branch))
}

fn extract_gitlab_tree_branch(remainder: &str) -> Option<String> {
    let after = remainder.strip_prefix("/-/")?;
    let parts: Vec<&str> = after.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() >= 2 && parts[0] == "tree" {
        let branch = parts[1].trim();
        if branch.is_empty() {
            None
        } else {
            Some(branch.to_string())
        }
    } else {
        None
    }
}

fn strip_url_query_and_fragment(input: &str) -> &str {
    input
        .split('?')
        .next()
        .unwrap_or(input)
        .split('#')
        .next()
        .unwrap_or(input)
}

pub fn get_skill_repos(config: &AppConfig) -> Vec<SkillRepo> {
    config.skill_repos.clone()
}

pub fn is_skill_repo_enabled(config: &AppConfig, host: &str, project_path: &str) -> bool {
    find_repo_index(config, host, project_path)
        .map(|index| config.skill_repos[index].enabled)
        .unwrap_or(true)
}

pub fn set_skill_repo_enabled(
    config: &mut AppConfig,
    host: &str,
    project_path: &str,
    enabled: bool,
) -> Result<SkillRepo, AppError> {
    let index = find_repo_index(config, host, project_path).ok_or_else(|| {
        let (owner, name) = split_owner_name(project_path);
        AppError::SkillRepoNotFound { owner, name }
    })?;
    config.skill_repos[index].enabled = enabled;
    Ok(config.skill_repos[index].clone())
}

/// Ensures built-in GitHub repos exist. Returns true when a repo was added.
pub fn ensure_builtin_repos(config: &mut AppConfig) -> bool {
    if find_repo_index(config, &default_github_host(), "obra/superpowers").is_some() {
        return false;
    }

    let _ = add_skill_repo(
        config,
        "https://github.com/obra/superpowers",
        None,
        None,
    );
    true
}

pub fn preview_add_skill_repo(_config: &AppConfig, url: &str) -> PreviewAddRepoResult {
    let parsed = match parse_repo_url(url) {
        Ok(parsed) => parsed,
        Err(err) => {
            return PreviewAddRepoResult {
                can_save: false,
                needs_pat: false,
                host: None,
                provider: None,
                project_path: None,
                branch: None,
                error: Some(err.to_dto()),
            };
        }
    };

    let mut result = PreviewAddRepoResult {
        can_save: false,
        needs_pat: false,
        host: Some(parsed.host.clone()),
        provider: Some(parsed.provider.clone()),
        project_path: Some(parsed.project_path.clone()),
        branch: Some(parsed.branch.clone()),
        error: None,
    };

    if parsed.provider == default_github_provider() {
        result.can_save = true;
        return result;
    }

    let token = credential_store::get_gitlab_token(&parsed.host)
        .ok()
        .flatten();

    let had_token = token.is_some();

    if !had_token {
        if let Err(err) = gitlab_client::probe_instance(&parsed.host) {
            result.error = Some(err.to_dto());
            return result;
        }
    }

    let probe_result =
        gitlab_client::probe_project_access(&parsed.host, &parsed.project_path, token.as_deref());
    apply_gitlab_probe_result(&mut result, probe_result, had_token);
    result
}

fn apply_gitlab_probe_result(
    result: &mut PreviewAddRepoResult,
    probe_result: Result<(), AppError>,
    had_token: bool,
) {
    match probe_result {
        Ok(()) => {
            result.can_save = true;
            result.needs_pat = false;
            result.error = None;
        }
        Err(AppError::GitLabAuthRequired { .. }) => {
            result.can_save = false;
            result.needs_pat = true;
            result.error = None;
        }
        Err(AppError::SkillRepoNotFound { .. }) if !had_token => {
            // GitLab 对未认证的私有仓库常返回 404，引导配置 PAT 而非报「找不到」
            result.can_save = false;
            result.needs_pat = true;
            result.error = None;
        }
        Err(err) => {
            result.can_save = false;
            result.needs_pat = false;
            result.error = Some(err.to_dto());
        }
    }
}

pub fn add_skill_repo(
    config: &mut AppConfig,
    url: &str,
    branch: Option<&str>,
    pat: Option<&str>,
) -> Result<SkillRepo, AppError> {
    let parsed = parse_repo_url(url)?;

    if let Some(pat) = pat.filter(|value| !value.trim().is_empty()) {
        gitlab_client::validate_token(&parsed.host, pat)?;
        credential_store::set_gitlab_token(&parsed.host, pat)?;
        credential_store::register_gitlab_host(config, &parsed.host);
    }

    let explicit_branch = branch
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let mut resolved_branch = explicit_branch
        .clone()
        .unwrap_or_else(|| parsed.branch.clone());

    if parsed.provider == "gitlab" && explicit_branch.is_none() {
        let token = pat
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .or_else(|| credential_store::get_gitlab_token(&parsed.host).ok().flatten());
        if let Some(token) = token {
            if let Ok(default_branch) =
                gitlab_client::fetch_default_branch(&parsed.host, &parsed.project_path, &token)
            {
                resolved_branch = default_branch;
            }
        }
    }

    if let Some(index) = find_repo_index(config, &parsed.host, &parsed.project_path) {
        return Ok(config.skill_repos[index].clone());
    }

    let repo = SkillRepo {
        host: parsed.host,
        provider: parsed.provider,
        project_path: parsed.project_path.clone(),
        owner: parsed.owner,
        name: parsed.name,
        branch: resolved_branch,
        enabled: true,
    };
    config.skill_repos.push(repo.clone());
    Ok(repo)
}

pub fn remove_skill_repo(
    config: &mut AppConfig,
    host: &str,
    project_path: &str,
) -> Result<(), AppError> {
    let index = find_repo_index(config, host, project_path).ok_or_else(|| {
        let (owner, name) = split_owner_name(project_path);
        AppError::SkillRepoNotFound { owner, name }
    })?;

    config.skill_repos.remove(index);
    Ok(())
}

fn extract_github_tree_branch(parts: &[&str]) -> Option<String> {
    if parts.len() >= 4 && parts[2] == "tree" {
        let branch = parts[3].trim();
        if branch.is_empty() {
            None
        } else {
            Some(branch.to_string())
        }
    } else {
        None
    }
}

fn resolve_repo_branch(input: &str, tree_branch: Option<String>) -> String {
    if let Some(branch) = tree_branch.filter(|value| !value.is_empty()) {
        return branch;
    }

    parse_branch_from_source_url(input).unwrap_or_else(|| "main".to_string())
}

fn parse_branch_from_source_url(source_url: &str) -> Option<String> {
    let source_url = source_url.trim();
    if source_url.is_empty() {
        return None;
    }

    if let Some((_, after)) = source_url.split_once("/-/tree/") {
        let branch = after
            .split('/')
            .next()
            .map(str::trim)
            .filter(|segment| !segment.is_empty())?;
        return Some(branch.to_string());
    }

    if let Some((_, after_tree)) = source_url.split_once("/tree/") {
        let branch = after_tree
            .split('/')
            .next()
            .map(str::trim)
            .filter(|segment| !segment.is_empty())?;
        return Some(branch.to_string());
    }

    if let Some((_, fragment)) = source_url.split_once('#') {
        let branch = fragment
            .split('&')
            .next()
            .map(str::trim)
            .filter(|segment| !segment.is_empty())?;
        return Some(branch.to_string());
    }

    if let Some((_, query)) = source_url.split_once('?') {
        for pair in query.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if matches!(key, "branch" | "ref") {
                let branch = value.trim();
                if !branch.is_empty() {
                    return Some(branch.to_string());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(url: &str) -> ParsedRepoUrl {
        parse_repo_url(url).expect("parse url")
    }

    #[test]
    fn parse_repo_url_accepts_https_github_url() {
        let parsed = parsed("https://github.com/anthropics/skills");

        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.provider, "github");
        assert_eq!(parsed.project_path, "anthropics/skills");
        assert_eq!(parsed.owner, "anthropics");
        assert_eq!(parsed.name, "skills");
        assert_eq!(parsed.branch, "main");
    }

    #[test]
    fn parse_repo_url_accepts_https_github_url_with_git_suffix() {
        let parsed = parsed("https://github.com/anthropics/skills.git");

        assert_eq!(parsed.project_path, "anthropics/skills");
        assert_eq!(parsed.branch, "main");
    }

    #[test]
    fn parse_repo_url_accepts_owner_name_shorthand() {
        let parsed = parsed("anthropics/skills");

        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.provider, "github");
        assert_eq!(parsed.project_path, "anthropics/skills");
        assert_eq!(parsed.branch, "main");
    }

    #[test]
    fn parse_repo_url_extracts_branch_from_tree_path() {
        let parsed = parsed(
            "https://github.com/anthropics/skills/tree/develop/skills/brainstorming",
        );

        assert_eq!(parsed.project_path, "anthropics/skills");
        assert_eq!(parsed.branch, "develop");
    }

    #[test]
    fn parse_repo_url_extracts_branch_from_query() {
        let parsed = parsed("https://github.com/anthropics/skills?branch=release");

        assert_eq!(parsed.project_path, "anthropics/skills");
        assert_eq!(parsed.branch, "release");
    }

    #[test]
    fn parse_repo_url_accepts_ssh_github_url() {
        let parsed = parsed("git@github.com:obra/superpowers.git");

        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.provider, "github");
        assert_eq!(parsed.project_path, "obra/superpowers");
        assert_eq!(parsed.branch, "main");
    }

    #[test]
    fn parse_repo_url_parses_gitlab_https_url() {
        let parsed = parsed("https://gitlab.example.com/acme/tools");

        assert_eq!(parsed.host, "gitlab.example.com");
        assert_eq!(parsed.provider, "gitlab");
        assert_eq!(parsed.project_path, "acme/tools");
        assert_eq!(parsed.owner, "acme");
        assert_eq!(parsed.name, "tools");
        assert_eq!(parsed.branch, "main");
    }

    #[test]
    fn parse_repo_url_parses_gitlab_multi_level_group() {
        let parsed = parsed("https://gitlab.example.com/group/subgroup/project");

        assert_eq!(parsed.host, "gitlab.example.com");
        assert_eq!(parsed.provider, "gitlab");
        assert_eq!(parsed.project_path, "group/subgroup/project");
        assert_eq!(parsed.owner, "group/subgroup");
        assert_eq!(parsed.name, "project");
    }

    #[test]
    fn parse_repo_url_parses_gitlab_tree_branch() {
        let parsed = parsed("https://gitlab.example.com/acme/tools/-/tree/develop/skills");

        assert_eq!(parsed.project_path, "acme/tools");
        assert_eq!(parsed.branch, "develop");
    }

    #[test]
    fn parse_repo_url_parses_gitlab_ssh_url() {
        let parsed = parsed("git@gitlab.example.com:acme/tools.git");

        assert_eq!(parsed.host, "gitlab.example.com");
        assert_eq!(parsed.provider, "gitlab");
        assert_eq!(parsed.project_path, "acme/tools");
    }

    #[test]
    fn parse_repo_url_parses_gitlab_url_without_scheme() {
        let parsed = parsed("gitlab.example.com/acme/tools");

        assert_eq!(parsed.host, "gitlab.example.com");
        assert_eq!(parsed.provider, "gitlab");
        assert_eq!(parsed.project_path, "acme/tools");
    }

    #[test]
    fn parse_repo_url_rejects_invalid_input() {
        let error = parse_repo_url("not-a-valid-repo").expect_err("invalid url");

        assert!(matches!(error, AppError::InvalidInput { .. }));
    }

    #[test]
    fn canonical_repo_key_uses_host_and_project_path() {
        assert_eq!(
            canonical_repo_key("GitLab.Example.COM", "Group/Project"),
            "gitlab.example.com/group/project"
        );
    }

    #[test]
    fn set_skill_repo_enabled_toggles_flag() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();
        add_skill_repo(&mut config, "anthropics/skills", None, None).expect("add repo");

        let repo = set_skill_repo_enabled(&mut config, "github.com", "anthropics/skills", false)
            .expect("disable repo");
        assert!(!repo.enabled);
        assert!(!is_skill_repo_enabled(&config, "github.com", "anthropics/skills"));

        let repo = set_skill_repo_enabled(&mut config, "github.com", "anthropics/skills", true)
            .expect("enable repo");
        assert!(repo.enabled);
    }

    #[test]
    fn add_skill_repo_appends_enabled_repo() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        let repo = add_skill_repo(&mut config, "anthropics/skills", None, None).expect("add repo");

        assert_eq!(repo.host, "github.com");
        assert_eq!(repo.provider, "github");
        assert_eq!(repo.project_path, "anthropics/skills");
        assert_eq!(repo.owner, "anthropics");
        assert_eq!(repo.name, "skills");
        assert_eq!(repo.branch, "main");
        assert!(repo.enabled);
        assert_eq!(config.skill_repos.len(), 1);
    }

    #[test]
    fn add_skill_repo_dedupes_by_host_and_project_path() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        add_skill_repo(&mut config, "anthropics/skills", None, None).expect("first add");
        add_skill_repo(
            &mut config,
            "https://github.com/anthropics/skills.git",
            None,
            None,
        )
        .expect("duplicate add");

        assert_eq!(config.skill_repos.len(), 1);
    }

    #[test]
    fn add_skill_repo_dedupes_case_insensitive_urls() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        add_skill_repo(&mut config, "Anthropics/Skills", None, None).expect("first add");
        add_skill_repo(
            &mut config,
            "https://www.github.com/ANTHROPICS/skills/",
            None,
            None,
        )
        .expect("duplicate add");

        assert_eq!(config.skill_repos.len(), 1);
        assert_eq!(config.skill_repos[0].project_path, "anthropics/skills");
    }

    #[test]
    fn add_skill_repo_dedupes_ssh_and_https_urls() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        add_skill_repo(
            &mut config,
            "git@github.com:anthropics/skills.git",
            None,
            None,
        )
        .expect("ssh add");
        add_skill_repo(
            &mut config,
            "https://github.com/anthropics/skills",
            None,
            None,
        )
        .expect("https duplicate");

        assert_eq!(config.skill_repos.len(), 1);
    }

    #[test]
    fn add_skill_repo_dedupes_gitlab_same_host_and_path() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        add_skill_repo(
            &mut config,
            "https://gitlab.example.com/acme/tools",
            None,
            None,
        )
        .expect("first add");
        add_skill_repo(
            &mut config,
            "git@gitlab.example.com:acme/tools.git",
            None,
            None,
        )
        .expect("duplicate add");

        assert_eq!(config.skill_repos.len(), 1);
        assert_eq!(config.skill_repos[0].host, "gitlab.example.com");
    }

    #[test]
    fn dedupe_skill_repos_removes_case_insensitive_duplicates() {
        let mut config = AppConfig::default();
        config.skill_repos = vec![
            SkillRepo {
                host: default_github_host(),
                provider: default_github_provider(),
                project_path: "anthropics/skills".to_string(),
                owner: "anthropics".to_string(),
                name: "skills".to_string(),
                branch: "main".to_string(),
                enabled: true,
            },
            SkillRepo {
                host: "GitHub.COM".to_string(),
                provider: default_github_provider(),
                project_path: "Anthropics/Skills".to_string(),
                owner: "Anthropics".to_string(),
                name: "Skills".to_string(),
                branch: "main".to_string(),
                enabled: true,
            },
            SkillRepo {
                host: default_github_host(),
                provider: default_github_provider(),
                project_path: "obra/superpowers".to_string(),
                owner: "obra".to_string(),
                name: "superpowers".to_string(),
                branch: "main".to_string(),
                enabled: true,
            },
        ];

        assert!(dedupe_skill_repos(&mut config));
        assert_eq!(config.skill_repos.len(), 2);
        assert_eq!(config.skill_repos[0].project_path, "anthropics/skills");
        assert_eq!(config.skill_repos[1].project_path, "obra/superpowers");
    }

    #[test]
    fn remove_skill_repo_matches_case_insensitively() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();
        add_skill_repo(&mut config, "anthropics/skills", None, None).expect("add repo");

        remove_skill_repo(&mut config, "GitHub.com", "Anthropics/Skills").expect("remove repo");

        assert!(config.skill_repos.is_empty());
    }

    #[test]
    fn add_skill_repo_uses_explicit_branch_override() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        let repo = add_skill_repo(&mut config, "anthropics/skills", Some("develop"), None)
            .expect("add with branch");

        assert_eq!(repo.branch, "develop");
    }

    #[test]
    fn remove_skill_repo_removes_matching_entry() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();
        add_skill_repo(&mut config, "anthropics/skills", None, None).expect("add repo");

        remove_skill_repo(&mut config, "github.com", "anthropics/skills").expect("remove repo");

        assert!(config.skill_repos.is_empty());
    }

    #[test]
    fn remove_skill_repo_does_not_touch_skill_records() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();
        add_skill_repo(&mut config, "anthropics/skills", None, None).expect("add repo");
        config.skill_records.insert(
            "brainstorming".to_string(),
            crate::models::SkillRecord {
                repo_host: default_github_host(),
                project_path: "anthropics/skills".to_string(),
                source: "github".to_string(),
                repo_owner: "anthropics".to_string(),
                repo_name: "skills".to_string(),
                repo_branch: "main".to_string(),
                directory: "skills/brainstorming".to_string(),
                content_hash: "abc".to_string(),
                installed_at: "2026-06-30T00:00:00Z".to_string(),
            },
        );

        remove_skill_repo(&mut config, "github.com", "anthropics/skills").expect("remove repo");

        assert!(config.skill_repos.is_empty());
        assert_eq!(config.skill_records.len(), 1);
    }

    #[test]
    fn ensure_builtin_repos_adds_superpowers_once() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        assert!(ensure_builtin_repos(&mut config));
        assert_eq!(config.skill_repos.len(), 1);
        assert_eq!(config.skill_repos[0].project_path, "obra/superpowers");
        assert_eq!(config.skill_repos[0].branch, "main");
        assert!(!ensure_builtin_repos(&mut config));
        assert_eq!(config.skill_repos.len(), 1);
    }

    #[test]
    fn remove_skill_repo_returns_error_when_missing() {
        let mut config = AppConfig::default();

        let error =
            remove_skill_repo(&mut config, "github.com", "anthropics/skills").expect_err("missing repo");

        assert!(matches!(error, AppError::SkillRepoNotFound { .. }));
    }

    #[test]
    fn preview_add_github_repo_can_save_without_probe() {
        let config = AppConfig::default();
        let result = preview_add_skill_repo(&config, "https://github.com/anthropics/skills");

        assert!(result.can_save);
        assert!(!result.needs_pat);
        assert_eq!(result.provider.as_deref(), Some("github"));
        assert_eq!(result.project_path.as_deref(), Some("anthropics/skills"));
        assert!(result.error.is_none());
    }

    #[test]
    fn preview_add_invalid_url_returns_error() {
        let config = AppConfig::default();
        let result = preview_add_skill_repo(&config, "not-a-valid-repo");

        assert!(!result.can_save);
        assert!(!result.needs_pat);
        assert!(result.error.is_some());
    }

    #[test]
    fn apply_gitlab_probe_result_maps_auth_required_to_needs_pat() {
        let mut result = PreviewAddRepoResult {
            can_save: false,
            needs_pat: false,
            host: Some("gitlab.example.com".to_string()),
            provider: Some("gitlab".to_string()),
            project_path: Some("acme/tools".to_string()),
            branch: Some("main".to_string()),
            error: None,
        };

        apply_gitlab_probe_result(
            &mut result,
            Err(AppError::GitLabAuthRequired {
                host: "gitlab.example.com".to_string(),
            }),
            false,
        );

        assert!(!result.can_save);
        assert!(result.needs_pat);
        assert!(result.error.is_none());
    }

    #[test]
    fn apply_gitlab_probe_result_maps_success_to_can_save() {
        let mut result = PreviewAddRepoResult {
            can_save: false,
            needs_pat: true,
            host: Some("gitlab.example.com".to_string()),
            provider: Some("gitlab".to_string()),
            project_path: Some("acme/tools".to_string()),
            branch: Some("main".to_string()),
            error: None,
        };

        apply_gitlab_probe_result(&mut result, Ok(()), true);

        assert!(result.can_save);
        assert!(!result.needs_pat);
        assert!(result.error.is_none());
    }

    #[test]
    fn apply_gitlab_probe_result_maps_invalid_token_to_error() {
        let mut result = PreviewAddRepoResult {
            can_save: false,
            needs_pat: false,
            host: Some("gitlab.example.com".to_string()),
            provider: Some("gitlab".to_string()),
            project_path: Some("acme/tools".to_string()),
            branch: Some("main".to_string()),
            error: None,
        };

        apply_gitlab_probe_result(
            &mut result,
            Err(AppError::GitLabAuthInvalid {
                host: "gitlab.example.com".to_string(),
            }),
            true,
        );

        assert!(!result.can_save);
        assert!(!result.needs_pat);
        assert_eq!(result.error.as_ref().map(|err| err.code.as_str()), Some("gitlabAuthInvalid"));
    }

    #[test]
    fn apply_gitlab_probe_result_maps_404_without_token_to_needs_pat() {
        let mut result = PreviewAddRepoResult {
            can_save: false,
            needs_pat: false,
            host: Some("gitlab.example.com".to_string()),
            provider: Some("gitlab".to_string()),
            project_path: Some("acme/tools/demo-skill".to_string()),
            branch: Some("main".to_string()),
            error: None,
        };

        apply_gitlab_probe_result(
            &mut result,
            Err(AppError::SkillRepoNotFound {
                owner: "acme/tools".to_string(),
                name: "demo-skill".to_string(),
            }),
            false,
        );

        assert!(!result.can_save);
        assert!(result.needs_pat);
        assert!(result.error.is_none());
    }

    #[test]
    fn apply_gitlab_probe_result_maps_404_with_token_to_error() {
        let mut result = PreviewAddRepoResult {
            can_save: false,
            needs_pat: false,
            host: Some("gitlab.example.com".to_string()),
            provider: Some("gitlab".to_string()),
            project_path: Some("acme/missing".to_string()),
            branch: Some("main".to_string()),
            error: None,
        };

        apply_gitlab_probe_result(
            &mut result,
            Err(AppError::SkillRepoNotFound {
                owner: "acme".to_string(),
                name: "missing".to_string(),
            }),
            true,
        );

        assert!(!result.can_save);
        assert!(!result.needs_pat);
        assert_eq!(result.error.as_ref().map(|err| err.code.as_str()), Some("skillRepoNotFound"));
    }
}
