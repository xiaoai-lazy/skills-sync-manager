use crate::models::{AppConfig, AppError, SkillRepo};
use std::collections::HashSet;

pub fn canonical_repo_key(owner: &str, name: &str) -> String {
    format!("{}/{}", owner.trim().to_lowercase(), name.trim().to_lowercase())
}

fn find_repo_index(config: &AppConfig, owner: &str, name: &str) -> Option<usize> {
    let key = canonical_repo_key(owner, name);
    config
        .skill_repos
        .iter()
        .position(|repo| canonical_repo_key(&repo.owner, &repo.name) == key)
}

/// Removes duplicate repos (same owner/name, case-insensitive). Returns true when entries were removed.
pub fn dedupe_skill_repos(config: &mut AppConfig) -> bool {
    let before = config.skill_repos.len();
    let mut seen = HashSet::new();
    config.skill_repos.retain(|repo| {
        seen.insert(canonical_repo_key(&repo.owner, &repo.name))
    });
    config.skill_repos.len() != before
}

pub fn parse_repo_url(url: &str) -> Result<(String, String, String), AppError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput {
            input: url.to_string(),
            message: "仓库 URL 不能为空".to_string(),
        });
    }

    let normalized = normalize_repo_input(trimmed);
    let path_only = strip_url_query_and_fragment(&normalized);
    let parts: Vec<&str> = path_only.split('/').filter(|part| !part.is_empty()).collect();

    if parts.len() < 2 {
        return Err(AppError::InvalidInput {
            input: url.to_string(),
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
            input: url.to_string(),
            message: "仓库 owner 或 name 不能为空".to_string(),
        });
    }

    let tree_branch = extract_tree_branch(&parts);
    let branch = resolve_repo_branch(trimmed, tree_branch);

    Ok((owner, name, branch))
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

/// Ensures built-in GitHub repos exist. Returns true when a repo was added.
pub fn ensure_builtin_repos(config: &mut AppConfig) -> bool {
    if find_repo_index(config, "obra", "superpowers").is_some() {
        return false;
    }

    let _ = add_skill_repo(config, "https://github.com/obra/superpowers", None);
    true
}

pub fn add_skill_repo(
    config: &mut AppConfig,
    url: &str,
    branch: Option<&str>,
) -> Result<SkillRepo, AppError> {
    let (owner, name, url_branch) = parse_repo_url(url)?;
    let branch = branch
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or(url_branch);

    if let Some(index) = find_repo_index(config, &owner, &name) {
        return Ok(config.skill_repos[index].clone());
    }

    let repo = SkillRepo {
        owner,
        name,
        branch,
        enabled: true,
    };
    config.skill_repos.push(repo.clone());
    Ok(repo)
}

pub fn remove_skill_repo(config: &mut AppConfig, owner: &str, name: &str) -> Result<(), AppError> {
    let index = find_repo_index(config, owner, name).ok_or_else(|| AppError::SkillRepoNotFound {
        owner: owner.to_string(),
        name: name.to_string(),
    })?;

    config.skill_repos.remove(index);
    Ok(())
}

fn normalize_repo_input(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(stripped) = trimmed.strip_prefix("git@github.com:") {
        return stripped.trim_end_matches('/').to_string();
    }

    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let without_host = without_scheme
        .strip_prefix("github.com/")
        .or_else(|| without_scheme.strip_prefix("www.github.com/"))
        .unwrap_or(without_scheme);
    without_host.trim_end_matches('/').to_string()
}

fn extract_tree_branch(parts: &[&str]) -> Option<String> {
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

    #[test]
    fn parse_repo_url_accepts_https_github_url() {
        let (owner, name, branch) =
            parse_repo_url("https://github.com/anthropics/skills").expect("parse https url");

        assert_eq!(owner, "anthropics");
        assert_eq!(name, "skills");
        assert_eq!(branch, "main");
    }

    #[test]
    fn parse_repo_url_accepts_https_github_url_with_git_suffix() {
        let (owner, name, branch) =
            parse_repo_url("https://github.com/anthropics/skills.git").expect("parse git suffix");

        assert_eq!(owner, "anthropics");
        assert_eq!(name, "skills");
        assert_eq!(branch, "main");
    }

    #[test]
    fn parse_repo_url_accepts_owner_name_shorthand() {
        let (owner, name, branch) =
            parse_repo_url("anthropics/skills").expect("parse owner/name");

        assert_eq!(owner, "anthropics");
        assert_eq!(name, "skills");
        assert_eq!(branch, "main");
    }

    #[test]
    fn parse_repo_url_extracts_branch_from_tree_path() {
        let (owner, name, branch) = parse_repo_url(
            "https://github.com/anthropics/skills/tree/develop/skills/brainstorming",
        )
        .expect("parse tree branch");

        assert_eq!(owner, "anthropics");
        assert_eq!(name, "skills");
        assert_eq!(branch, "develop");
    }

    #[test]
    fn parse_repo_url_extracts_branch_from_query() {
        let (owner, name, branch) =
            parse_repo_url("https://github.com/anthropics/skills?branch=release").expect("parse query");

        assert_eq!(owner, "anthropics");
        assert_eq!(name, "skills");
        assert_eq!(branch, "release");
    }

    #[test]
    fn parse_repo_url_rejects_invalid_input() {
        let error = parse_repo_url("not-a-valid-repo").expect_err("invalid url");

        assert!(matches!(error, AppError::InvalidInput { .. }));
    }

    #[test]
    fn add_skill_repo_appends_enabled_repo() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        let repo = add_skill_repo(&mut config, "anthropics/skills", None).expect("add repo");

        assert_eq!(repo.owner, "anthropics");
        assert_eq!(repo.name, "skills");
        assert_eq!(repo.branch, "main");
        assert!(repo.enabled);
        assert_eq!(config.skill_repos.len(), 1);
    }

    #[test]
    fn add_skill_repo_dedupes_by_owner_and_name() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        add_skill_repo(&mut config, "anthropics/skills", None).expect("first add");
        add_skill_repo(&mut config, "https://github.com/anthropics/skills.git", None)
            .expect("duplicate add");

        assert_eq!(config.skill_repos.len(), 1);
    }

    #[test]
    fn add_skill_repo_dedupes_case_insensitive_urls() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        add_skill_repo(&mut config, "Anthropics/Skills", None).expect("first add");
        add_skill_repo(
            &mut config,
            "https://www.github.com/ANTHROPICS/skills/",
            None,
        )
        .expect("duplicate add");

        assert_eq!(config.skill_repos.len(), 1);
        assert_eq!(config.skill_repos[0].owner, "anthropics");
        assert_eq!(config.skill_repos[0].name, "skills");
    }

    #[test]
    fn add_skill_repo_dedupes_ssh_and_https_urls() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        add_skill_repo(&mut config, "git@github.com:anthropics/skills.git", None)
            .expect("ssh add");
        add_skill_repo(&mut config, "https://github.com/anthropics/skills", None)
            .expect("https duplicate");

        assert_eq!(config.skill_repos.len(), 1);
    }

    #[test]
    fn dedupe_skill_repos_removes_case_insensitive_duplicates() {
        let mut config = AppConfig::default();
        config.skill_repos = vec![
            SkillRepo {
                owner: "anthropics".to_string(),
                name: "skills".to_string(),
                branch: "main".to_string(),
                enabled: true,
            },
            SkillRepo {
                owner: "Anthropics".to_string(),
                name: "Skills".to_string(),
                branch: "main".to_string(),
                enabled: true,
            },
            SkillRepo {
                owner: "obra".to_string(),
                name: "superpowers".to_string(),
                branch: "main".to_string(),
                enabled: true,
            },
        ];

        assert!(dedupe_skill_repos(&mut config));
        assert_eq!(config.skill_repos.len(), 2);
        assert_eq!(config.skill_repos[0].owner, "anthropics");
        assert_eq!(config.skill_repos[1].owner, "obra");
    }

    #[test]
    fn remove_skill_repo_matches_case_insensitively() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();
        add_skill_repo(&mut config, "anthropics/skills", None).expect("add repo");

        remove_skill_repo(&mut config, "Anthropics", "Skills").expect("remove repo");

        assert!(config.skill_repos.is_empty());
    }

    #[test]
    fn parse_repo_url_accepts_ssh_github_url() {
        let (owner, name, branch) =
            parse_repo_url("git@github.com:obra/superpowers.git").expect("parse ssh url");

        assert_eq!(owner, "obra");
        assert_eq!(name, "superpowers");
        assert_eq!(branch, "main");
    }

    #[test]
    fn add_skill_repo_uses_explicit_branch_override() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        let repo = add_skill_repo(&mut config, "anthropics/skills", Some("develop"))
            .expect("add with branch");

        assert_eq!(repo.branch, "develop");
    }

    #[test]
    fn remove_skill_repo_removes_matching_entry() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();
        add_skill_repo(&mut config, "anthropics/skills", None).expect("add repo");

        remove_skill_repo(&mut config, "anthropics", "skills").expect("remove repo");

        assert!(config.skill_repos.is_empty());
    }

    #[test]
    fn remove_skill_repo_does_not_touch_skill_records() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();
        add_skill_repo(&mut config, "anthropics/skills", None).expect("add repo");
        config.skill_records.insert(
            "brainstorming".to_string(),
            crate::models::SkillRecord {
                source: "github".to_string(),
                repo_owner: "anthropics".to_string(),
                repo_name: "skills".to_string(),
                repo_branch: "main".to_string(),
                directory: "skills/brainstorming".to_string(),
                content_hash: "abc".to_string(),
                installed_at: "2026-06-30T00:00:00Z".to_string(),
            },
        );

        remove_skill_repo(&mut config, "anthropics", "skills").expect("remove repo");

        assert!(config.skill_repos.is_empty());
        assert_eq!(config.skill_records.len(), 1);
    }

    #[test]
    fn ensure_builtin_repos_adds_superpowers_once() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();

        assert!(ensure_builtin_repos(&mut config));
        assert_eq!(config.skill_repos.len(), 1);
        assert_eq!(config.skill_repos[0].owner, "obra");
        assert_eq!(config.skill_repos[0].name, "superpowers");
        assert_eq!(config.skill_repos[0].branch, "main");
        assert!(!ensure_builtin_repos(&mut config));
        assert_eq!(config.skill_repos.len(), 1);
    }

    #[test]
    fn remove_skill_repo_returns_error_when_missing() {
        let mut config = AppConfig::default();

        let error = remove_skill_repo(&mut config, "anthropics", "skills").expect_err("missing repo");

        assert!(matches!(error, AppError::SkillRepoNotFound { .. }));
    }
}
