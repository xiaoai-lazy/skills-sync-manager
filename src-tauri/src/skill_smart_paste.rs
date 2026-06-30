use crate::credential_store;
use crate::gitlab_client;
use crate::models::{
    default_github_host, AppError, DiscoverableSkill, SmartPastePreview,
};
use crate::skill_library;
use reqwest::Url;
use serde::Deserialize;

struct ParsedGitHub {
    owner: String,
    repo: String,
    branch: String,
    directory: String,
}

pub(crate) struct ParsedGitLab {
    repo_host: String,
    project_path: String,
    branch: String,
    directory: String,
}

pub fn parse_smart_paste(input: &str) -> Result<SmartPastePreview, AppError> {
    parse_smart_paste_with_hooks(input, search_skills_sh, fetch_gitlab_skill_md)
}

pub fn parse_smart_paste_with_search<F>(
    input: &str,
    search: F,
) -> Result<SmartPastePreview, AppError>
where
    F: FnOnce(&str, u32, u32) -> Result<Vec<DiscoverableSkill>, AppError>,
{
    parse_smart_paste_with_hooks(input, search, fetch_gitlab_skill_md)
}

pub(crate) fn parse_smart_paste_with_hooks<F, G>(
    input: &str,
    search: F,
    gitlab_fetch: G,
) -> Result<SmartPastePreview, AppError>
where
    F: FnOnce(&str, u32, u32) -> Result<Vec<DiscoverableSkill>, AppError>,
    G: FnOnce(&ParsedGitLab) -> Result<String, AppError>,
{
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput {
            input: input.to_string(),
            message: "输入为空".to_string(),
        });
    }

    if is_skills_sh_input(trimmed) {
        return parse_skills_sh_input(trimmed, search);
    }

    if looks_like_github_input(trimmed) {
        let parsed = parse_github_input(trimmed)?;
        return Ok(github_to_preview(parsed));
    }

    if looks_like_gitlab_input(trimmed) {
        let parsed = parse_gitlab_input(trimmed)?;
        return gitlab_to_preview_with_fetch(parsed, gitlab_fetch);
    }

    Err(AppError::InvalidInput {
        input: trimmed.to_string(),
        message: "不支持的链接格式".to_string(),
    })
}

pub fn search_skills_sh(
    query: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<DiscoverableSkill>, AppError> {
    search_skills_sh_with_fetch(query, limit, offset, fetch_skills_sh_search)
}

pub fn search_skills_sh_with_fetch<F>(
    query: &str,
    limit: u32,
    offset: u32,
    fetch: F,
) -> Result<Vec<DiscoverableSkill>, AppError>
where
    F: FnOnce(&str, u32, u32) -> Result<String, AppError>,
{
    let body = fetch(query, limit, offset)?;
    parse_skills_sh_search_response(&body)
}

fn fetch_skills_sh_search(query: &str, limit: u32, offset: u32) -> Result<String, AppError> {
    let url = skills_sh_search_url(query, limit, offset)?;
    let response = reqwest::blocking::get(url.as_str()).map_err(|err| AppError::Io {
        path: None,
        message: format!("请求 skills.sh 失败: {}", err),
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(AppError::DownloadFailed {
            url: url.to_string(),
            status: Some(status.as_u16()),
            message: format!("skills.sh 搜索失败，HTTP 状态码 {}", status.as_u16()),
        });
    }

    response.text().map_err(|err| AppError::Io {
        path: None,
        message: format!("读取 skills.sh 响应失败: {}", err),
    })
}

fn skills_sh_search_url(query: &str, limit: u32, offset: u32) -> Result<Url, AppError> {
    let mut url = Url::parse("https://skills.sh/api/search").map_err(|err| AppError::Io {
        path: None,
        message: format!("构建 skills.sh 搜索 URL 失败: {}", err),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("q", query);
        pairs.append_pair("limit", &limit.to_string());
        pairs.append_pair("offset", &offset.to_string());
    }
    Ok(url)
}

#[derive(Debug, Deserialize)]
struct SkillsShApiResponse {
    skills: Vec<SkillsShApiSkill>,
}

#[derive(Debug, Deserialize)]
struct SkillsShApiSkill {
    id: String,
    #[serde(rename = "skillId")]
    skill_id: String,
    name: String,
    source: String,
}

fn parse_skills_sh_search_response(body: &str) -> Result<Vec<DiscoverableSkill>, AppError> {
    let response: SkillsShApiResponse =
        serde_json::from_str(body).map_err(|err| AppError::Io {
            path: None,
            message: format!("解析 skills.sh 响应失败: {}", err),
        })?;

    Ok(response
        .skills
        .into_iter()
        .filter_map(map_skills_sh_api_skill)
        .collect())
}

fn map_skills_sh_api_skill(skill: SkillsShApiSkill) -> Option<DiscoverableSkill> {
    let parts: Vec<&str> = skill.source.splitn(2, '/').collect();
    if parts.len() != 2 {
        return None;
    }

    let owner = parts[0];
    let repo = parts[1];
    if owner.contains('.') || repo.contains('.') {
        return None;
    }

    Some(DiscoverableSkill {
        key: skill.id,
        name: skill.name,
        description: String::new(),
        directory: skill.skill_id.clone(),
        install_dir_name: install_dir_name_from_directory(&skill.skill_id),
        repo_host: "github.com".to_string(),
        project_path: format!("{}/{}", owner, repo),
        repo_owner: owner.to_string(),
        repo_name: repo.to_string(),
        repo_branch: "main".to_string(),
        source: "skillssh".to_string(),
    })
}

fn parse_skills_sh_input<F>(
    input: &str,
    search: F,
) -> Result<SmartPastePreview, AppError>
where
    F: FnOnce(&str, u32, u32) -> Result<Vec<DiscoverableSkill>, AppError>,
{
    if let Some(id) = parse_skills_sh_id(input) {
        return Ok(skills_sh_id_to_preview(id));
    }

    let query = extract_skills_sh_query(input).ok_or_else(|| AppError::InvalidInput {
        input: input.to_string(),
        message: "无法从 skills.sh 链接中提取 Skill 标识".to_string(),
    })?;

    let results = search(&query, 5, 0)?;
    let skill = results
        .iter()
        .find(|item| item.name == query || item.install_dir_name == query)
        .or_else(|| results.first())
        .ok_or_else(|| AppError::InvalidInput {
            input: input.to_string(),
            message: format!("skills.sh 未找到 Skill '{}'", query),
        })?;

    Ok(discoverable_to_preview(skill))
}

fn parse_skills_sh_id(input: &str) -> Option<SkillsShId> {
    let trimmed = input.trim();
    if trimmed.contains("skills.sh") {
        return None;
    }

    let parts: Vec<&str> = trimmed.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() != 3 {
        return None;
    }

    let owner = parts[0];
    let repo = parts[1];
    let skill_id = parts[2];
    if owner.contains('.') || repo.contains('.') {
        return None;
    }

    Some(SkillsShId {
        owner: owner.to_string(),
        repo: repo.to_string(),
        skill_id: skill_id.to_string(),
    })
}

struct SkillsShId {
    owner: String,
    repo: String,
    skill_id: String,
}

fn skills_sh_id_to_preview(id: SkillsShId) -> SmartPastePreview {
    SmartPastePreview {
        name: id.skill_id.clone(),
        description: String::new(),
        install_dir_name: install_dir_name_from_directory(&id.skill_id),
        repo_host: default_github_host(),
        project_path: format!("{}/{}", id.owner, id.repo),
        repo_owner: id.owner,
        repo_name: id.repo,
        repo_branch: "main".to_string(),
        directory: id.skill_id,
        source: "skillssh".to_string(),
    }
}

fn extract_skills_sh_query(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let path = without_scheme
        .strip_prefix("skills.sh/")
        .or_else(|| without_scheme.strip_prefix("www.skills.sh/"))?;
    let segment = path.split('/').filter(|part| !part.is_empty()).last()?;
    if segment.is_empty() {
        None
    } else {
        Some(segment.to_string())
    }
}

fn is_skills_sh_input(input: &str) -> bool {
    let lower = input.trim().to_lowercase();
    lower.contains("skills.sh") || parse_skills_sh_id(input).is_some()
}

fn looks_like_github_input(input: &str) -> bool {
    let lower = input.trim().to_lowercase();
    lower.contains("github.com/") || is_owner_repo_path(input)
}

fn looks_like_gitlab_input(input: &str) -> bool {
    let trimmed = input.trim();
    if is_skills_sh_input(trimmed) || looks_like_github_input(trimmed) {
        return false;
    }

    let lower = trimmed.to_lowercase();
    if lower.contains("/-/blob/") || lower.contains("/-/tree/") {
        return true;
    }

    if let Some(after_git) = trimmed.strip_prefix("git@") {
        if let Some((host, _)) = after_git.split_once(':') {
            return host.contains('.') && !host.eq_ignore_ascii_case("github.com");
        }
    }

    if trimmed.contains("://") {
        if let Ok(url) = Url::parse(strip_url_query_and_fragment(trimmed)) {
            if let Some(host) = url.host_str() {
                let normalized = normalize_gitlab_host(host);
                return normalized.contains('.') && normalized != "github.com";
            }
        }
    }

    let path_only = strip_url_query_and_fragment(trimmed);
    if let Some((first, rest)) = path_only.split_once('/') {
        if first.contains('.') && !first.eq_ignore_ascii_case("github.com") && !rest.is_empty() {
            return true;
        }
    }

    false
}

fn is_owner_repo_path(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.contains("://") || trimmed.contains("skills.sh") {
        return false;
    }

    let parts: Vec<&str> = trimmed.split('/').filter(|part| !part.is_empty()).collect();
    parts.len() >= 2
        && parts[0] != "tree"
        && parts[0] != "blob"
        && !parts[0].contains('.')
}

fn parse_github_input(input: &str) -> Result<ParsedGitHub, AppError> {
    let path_only = strip_url_query_and_fragment(input);
    let normalized = normalize_github_input(path_only);
    let parts: Vec<&str> = normalized.split('/').filter(|part| !part.is_empty()).collect();

    if parts.len() < 2 {
        return Err(AppError::InvalidInput {
            input: input.to_string(),
            message: "GitHub 链接格式无效".to_string(),
        });
    }

    let owner = parts[0].to_string();
    let repo = parts[1]
        .strip_suffix(".git")
        .unwrap_or(parts[1])
        .to_string();

    if parts.len() == 2 {
        return Err(AppError::MissingSkillPath {
            input: input.to_string(),
        });
    }

    if parts.len() >= 4 && parts[2] == "tree" {
        let branch = parts[3].to_string();
        let directory_parts = &parts[4..];
        if directory_parts.is_empty() {
            return Err(AppError::MissingSkillPath {
                input: input.to_string(),
            });
        }

        return Ok(ParsedGitHub {
            owner,
            repo,
            branch: resolve_branch(input, Some(branch)),
            directory: directory_parts.join("/"),
        });
    }

    if parts.len() >= 4 && parts[2] == "blob" {
        let branch = parts[3].to_string();
        let path_parts = &parts[4..];
        if path_parts.is_empty() {
            return Err(AppError::MissingSkillPath {
                input: input.to_string(),
            });
        }

        let mut directory = path_parts.join("/");
        if directory.ends_with("/SKILL.md") {
            directory = directory
                .strip_suffix("/SKILL.md")
                .unwrap_or(&directory)
                .to_string();
        } else if directory == "SKILL.md" {
            return Err(AppError::MissingSkillPath {
                input: input.to_string(),
            });
        } else if !directory.ends_with("SKILL.md") {
            return Err(AppError::InvalidInput {
                input: input.to_string(),
                message: "GitHub blob 链接必须指向 SKILL.md 文件".to_string(),
            });
        } else {
            directory = directory
                .strip_suffix("SKILL.md")
                .unwrap_or(&directory)
                .trim_end_matches('/')
                .to_string();
        }

        return Ok(ParsedGitHub {
            owner,
            repo,
            branch: resolve_branch(input, Some(branch)),
            directory,
        });
    }

    if parts.len() >= 3 {
        return Ok(ParsedGitHub {
            owner,
            repo,
            branch: resolve_branch(input, None),
            directory: parts[2..].join("/"),
        });
    }

    Err(AppError::InvalidInput {
        input: input.to_string(),
        message: "GitHub 链接格式无效".to_string(),
    })
}

fn parse_gitlab_input(input: &str) -> Result<ParsedGitLab, AppError> {
    let (repo_host, path) = extract_gitlab_host_and_path(input)?;
    let path_only = strip_url_query_and_fragment(&path);

    let (project_segment, remainder) = if let Some(index) = path_only.find("/-/") {
        (&path_only[..index], Some(&path_only[index + 3..]))
    } else {
        (path_only.as_ref(), None)
    };

    let project_path = project_segment
        .strip_suffix(".git")
        .unwrap_or(project_segment)
        .trim()
        .trim_end_matches('/')
        .to_string();

    if project_path.is_empty() {
        return Err(AppError::InvalidInput {
            input: input.to_string(),
            message: "GitLab 项目路径不能为空".to_string(),
        });
    }

    let Some(remainder) = remainder else {
        return Err(AppError::MissingSkillPath {
            input: input.to_string(),
        });
    };

    let parts: Vec<&str> = remainder.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() < 3 {
        return Err(AppError::MissingSkillPath {
            input: input.to_string(),
        });
    }

    let branch = parts[1].to_string();
    if branch.is_empty() {
        return Err(AppError::MissingSkillPath {
            input: input.to_string(),
        });
    }

    let directory = if parts[0] == "tree" {
        let directory_parts = &parts[2..];
        if directory_parts.is_empty() {
            return Err(AppError::MissingSkillPath {
                input: input.to_string(),
            });
        }
        directory_parts.join("/")
    } else if parts[0] == "blob" {
        let path_parts = &parts[2..];
        if path_parts.is_empty() {
            return Err(AppError::MissingSkillPath {
                input: input.to_string(),
            });
        }

        let mut directory = path_parts.join("/");
        if directory.ends_with("/SKILL.md") {
            directory = directory
                .strip_suffix("/SKILL.md")
                .unwrap_or(&directory)
                .to_string();
        } else if directory == "SKILL.md" {
            return Err(AppError::MissingSkillPath {
                input: input.to_string(),
            });
        } else if !directory.ends_with("SKILL.md") {
            return Err(AppError::InvalidInput {
                input: input.to_string(),
                message: "GitLab blob 链接必须指向 SKILL.md 文件".to_string(),
            });
        } else {
            directory = directory
                .strip_suffix("SKILL.md")
                .unwrap_or(&directory)
                .trim_end_matches('/')
                .to_string();
        }
        directory
    } else {
        return Err(AppError::InvalidInput {
            input: input.to_string(),
            message: "GitLab 链接格式无效，需要 /-/tree/ 或 /-/blob/".to_string(),
        });
    };

    Ok(ParsedGitLab {
        repo_host,
        project_path,
        branch: resolve_branch(input, Some(branch)),
        directory,
    })
}

fn extract_gitlab_host_and_path(input: &str) -> Result<(String, String), AppError> {
    let trimmed = input.trim();

    if let Some(after_git) = trimmed.strip_prefix("git@") {
        let (host, path) = after_git.split_once(':').ok_or_else(|| AppError::InvalidInput {
            input: input.to_string(),
            message: "SSH GitLab 链接格式无效".to_string(),
        })?;
        let path = path
            .strip_suffix(".git")
            .unwrap_or(path)
            .trim_end_matches('/')
            .to_string();
        return Ok((normalize_gitlab_host(host), path));
    }

    if let Some(after_scheme) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
    {
        let path_only = strip_url_query_and_fragment(after_scheme);
        let (host, path) = path_only.split_once('/').ok_or_else(|| AppError::InvalidInput {
            input: input.to_string(),
            message: "GitLab 链接格式无效，缺少项目路径".to_string(),
        })?;
        return Ok((
            normalize_gitlab_host(host),
            path.trim_end_matches('/').to_string(),
        ));
    }

    let path_only = strip_url_query_and_fragment(trimmed);
    if let Some((first, rest)) = path_only.split_once('/') {
        if first.contains('.') {
            return Ok((
                normalize_gitlab_host(first),
                rest.trim_end_matches('/').to_string(),
            ));
        }
    }

    Err(AppError::InvalidInput {
        input: input.to_string(),
        message: "GitLab 链接格式无效".to_string(),
    })
}

fn normalize_gitlab_host(host: &str) -> String {
    host.trim()
        .trim_end_matches('/')
        .strip_prefix("www.")
        .unwrap_or(host.trim())
        .to_lowercase()
}

fn normalize_github_input(input: &str) -> String {
    let trimmed = input.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let without_host = without_scheme
        .strip_prefix("github.com/")
        .unwrap_or(without_scheme);
    without_host.to_string()
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

fn resolve_branch(input: &str, tree_branch: Option<String>) -> String {
    if let Some(branch) = tree_branch {
        if !branch.is_empty() {
            return branch;
        }
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

    if let Some((_, after_blob)) = source_url.split_once("/-/blob/") {
        let branch = after_blob
            .split('/')
            .next()
            .map(str::trim)
            .filter(|segment| !segment.is_empty())?;
        return Some(branch.to_string());
    }

    if let Some((_, after_tree)) = source_url.split_once("/-/tree/") {
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

fn github_to_preview(parsed: ParsedGitHub) -> SmartPastePreview {
    SmartPastePreview {
        name: install_dir_name_from_directory(&parsed.directory),
        description: String::new(),
        install_dir_name: install_dir_name_from_directory(&parsed.directory),
        repo_host: default_github_host(),
        project_path: format!("{}/{}", parsed.owner, parsed.repo),
        repo_owner: parsed.owner,
        repo_name: parsed.repo,
        repo_branch: parsed.branch,
        directory: parsed.directory,
        source: "github".to_string(),
    }
}

fn gitlab_to_preview_with_fetch<F>(
    parsed: ParsedGitLab,
    fetch: F,
) -> Result<SmartPastePreview, AppError>
where
    F: FnOnce(&ParsedGitLab) -> Result<String, AppError>,
{
    let raw = fetch(&parsed)?;
    let (name, description) =
        if let Some(metadata) = skill_library::parse_valid_skill_metadata(&raw) {
            (metadata.name, metadata.description)
        } else {
            let fallback = install_dir_name_from_directory(&parsed.directory);
            (fallback.clone(), String::new())
        };

    let (repo_owner, repo_name) = project_path_to_owner_name(&parsed.project_path);

    Ok(SmartPastePreview {
        name,
        description,
        install_dir_name: install_dir_name_from_directory(&parsed.directory),
        repo_host: parsed.repo_host,
        project_path: parsed.project_path,
        repo_owner,
        repo_name,
        repo_branch: parsed.branch,
        directory: parsed.directory,
        source: "gitlab".to_string(),
    })
}

fn fetch_gitlab_skill_md(parsed: &ParsedGitLab) -> Result<String, AppError> {
    let token = credential_store::get_gitlab_token(&parsed.repo_host)?;
    let skill_md_path = format!("{}/SKILL.md", parsed.directory);
    gitlab_client::fetch_file_raw(
        &parsed.repo_host,
        &parsed.project_path,
        &skill_md_path,
        &parsed.branch,
        token.as_deref(),
    )
}

fn discoverable_to_preview(skill: &DiscoverableSkill) -> SmartPastePreview {
    SmartPastePreview {
        name: skill.name.clone(),
        description: skill.description.clone(),
        install_dir_name: skill.install_dir_name.clone(),
        repo_host: if skill.repo_host.is_empty() {
            default_github_host()
        } else {
            skill.repo_host.clone()
        },
        project_path: if skill.project_path.is_empty() {
            format!("{}/{}", skill.repo_owner, skill.repo_name)
        } else {
            skill.project_path.clone()
        },
        repo_owner: skill.repo_owner.clone(),
        repo_name: skill.repo_name.clone(),
        repo_branch: skill.repo_branch.clone(),
        directory: skill.directory.clone(),
        source: skill.source.clone(),
    }
}

fn project_path_to_owner_name(project_path: &str) -> (String, String) {
    match project_path.rsplit_once('/') {
        Some((owner, name)) => (owner.to_string(), name.to_string()),
        None => (String::new(), project_path.to_string()),
    }
}

fn install_dir_name_from_directory(directory: &str) -> String {
    directory
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(directory)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOCK_SEARCH_JSON: &str = r#"{
        "query": "brainstorming",
        "searchType": "fuzzy",
        "skills": [
            {
                "id": "obra/superpowers/brainstorming",
                "skillId": "brainstorming",
                "name": "brainstorming",
                "installs": 100,
                "source": "obra/superpowers"
            },
            {
                "id": "skills.volces.com/example/foo",
                "skillId": "foo",
                "name": "foo",
                "installs": 1,
                "source": "skills.volces.com/example"
            }
        ],
        "count": 2,
        "duration_ms": 1
    }"#;

    const MOCK_GITLAB_SKILL_MD: &str = r#"---
name: demo-skill
description: Demo skill for tests.
---
"#;

    #[test]
    fn parse_tree_url_extracts_owner_repo_branch_and_directory() {
        let preview = parse_smart_paste(
            "https://github.com/anthropics/skills/tree/main/skills/brainstorming",
        )
        .expect("tree url parses");

        assert_eq!(preview.repo_host, "github.com");
        assert_eq!(preview.project_path, "anthropics/skills");
        assert_eq!(preview.repo_owner, "anthropics");
        assert_eq!(preview.repo_name, "skills");
        assert_eq!(preview.repo_branch, "main");
        assert_eq!(preview.directory, "skills/brainstorming");
        assert_eq!(preview.install_dir_name, "brainstorming");
        assert_eq!(preview.name, "brainstorming");
        assert_eq!(preview.source, "github");
    }

    #[test]
    fn parse_blob_url_strips_skill_md_from_directory() {
        let preview = parse_smart_paste(
            "https://github.com/anthropics/skills/blob/main/skills/brainstorming/SKILL.md",
        )
        .expect("blob url parses");

        assert_eq!(preview.directory, "skills/brainstorming");
        assert_eq!(preview.install_dir_name, "brainstorming");
        assert_eq!(preview.repo_branch, "main");
        assert_eq!(preview.repo_host, "github.com");
        assert_eq!(preview.project_path, "anthropics/skills");
    }

    #[test]
    fn parse_owner_repo_only_returns_missing_skill_path() {
        let error = parse_smart_paste("anthropics/skills").expect_err("owner/repo should fail");

        assert!(matches!(error, AppError::MissingSkillPath { .. }));
        assert_eq!(error.to_dto().code, "missingSkillPath");
        assert!(error.to_dto().message.contains(crate::models::SMART_PASTE_GITHUB_EXAMPLE));
    }

    #[test]
    fn parse_tree_url_without_directory_returns_missing_skill_path() {
        let error = parse_smart_paste("https://github.com/anthropics/skills/tree/main")
            .expect_err("tree without directory should fail");

        assert!(matches!(error, AppError::MissingSkillPath { .. }));
    }

    #[test]
    fn parse_branch_from_query_parameter() {
        let preview = parse_smart_paste("anthropics/skills/skills/brainstorming?ref=dev")
            .expect("query branch parses");

        assert_eq!(preview.repo_branch, "dev");
        assert_eq!(preview.directory, "skills/brainstorming");
    }

    #[test]
    fn parse_skills_sh_id_without_network() {
        let preview =
            parse_smart_paste("obra/superpowers/brainstorming").expect("skills.sh id parses");

        assert_eq!(preview.repo_host, "github.com");
        assert_eq!(preview.project_path, "obra/superpowers");
        assert_eq!(preview.repo_owner, "obra");
        assert_eq!(preview.repo_name, "superpowers");
        assert_eq!(preview.directory, "brainstorming");
        assert_eq!(preview.source, "skillssh");
    }

    #[test]
    fn search_skills_sh_parses_mock_response_and_filters_non_github_sources() {
        let skills = search_skills_sh_with_fetch("brainstorming", 10, 0, |_, _, _| {
            Ok(MOCK_SEARCH_JSON.to_string())
        })
        .expect("mock search parses");

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].key, "obra/superpowers/brainstorming");
        assert_eq!(skills[0].repo_owner, "obra");
        assert_eq!(skills[0].repo_name, "superpowers");
        assert_eq!(skills[0].source, "skillssh");
        assert_eq!(skills[0].install_dir_name, "brainstorming");
    }

    #[test]
    fn parse_skills_sh_url_uses_search_hook() {
        let preview = parse_smart_paste_with_search(
            "https://skills.sh/skills/brainstorming",
            |query, limit, offset| {
                assert_eq!(query, "brainstorming");
                assert_eq!(limit, 5);
                assert_eq!(offset, 0);
                search_skills_sh_with_fetch(query, limit, offset, |_, _, _| {
                    Ok(MOCK_SEARCH_JSON.to_string())
                })
            },
        )
        .expect("skills.sh url resolves through hook");

        assert_eq!(preview.name, "brainstorming");
        assert_eq!(preview.repo_owner, "obra");
        assert_eq!(preview.source, "skillssh");
    }

    #[test]
    fn parse_gitlab_blob_skill_md() {
        let preview = parse_smart_paste_with_hooks(
            "https://gitlab.example.com/acme/tools/-/blob/main/demo-skill/SKILL.md",
            |_, _, _| Ok(vec![]),
            |_| Ok(MOCK_GITLAB_SKILL_MD.to_string()),
        )
        .expect("gitlab blob parses");

        assert_eq!(preview.repo_host, "gitlab.example.com");
        assert_eq!(preview.project_path, "acme/tools");
        assert_eq!(preview.repo_branch, "main");
        assert_eq!(preview.directory, "demo-skill");
        assert_eq!(preview.install_dir_name, "demo-skill");
        assert_eq!(preview.name, "demo-skill");
        assert_eq!(preview.description, "Demo skill for tests.");
        assert_eq!(preview.source, "gitlab");
        assert_eq!(preview.repo_owner, "acme");
        assert_eq!(preview.repo_name, "tools");
    }

    #[test]
    fn parse_gitlab_tree_directory() {
        let preview = parse_smart_paste_with_hooks(
            "https://gitlab.example.com/acme/tools/-/tree/main/demo-skill",
            |_, _, _| Ok(vec![]),
            |_| Ok(MOCK_GITLAB_SKILL_MD.to_string()),
        )
        .expect("gitlab tree parses");

        assert_eq!(preview.repo_host, "gitlab.example.com");
        assert_eq!(preview.project_path, "acme/tools");
        assert_eq!(preview.repo_branch, "main");
        assert_eq!(preview.directory, "demo-skill");
        assert_eq!(preview.source, "gitlab");
        assert_eq!(preview.name, "demo-skill");
    }

    #[test]
    fn gitlab_auth_required_without_token() {
        let parsed = parse_gitlab_input(
            "https://gitlab.example.com/acme/tools/-/blob/main/demo-skill/SKILL.md",
        )
        .expect("gitlab input parses");

        let error = gitlab_to_preview_with_fetch(parsed, |_| {
            Err(AppError::GitLabAuthRequired {
                host: "gitlab.example.com".to_string(),
            })
        })
        .expect_err("auth required");

        assert!(matches!(error, AppError::GitLabAuthRequired { .. }));
        assert_eq!(error.to_dto().code, "gitlabAuthRequired");
        assert!(error.to_dto().message.contains("gitlab.example.com"));
    }

    #[test]
    fn looks_like_gitlab_input_detects_custom_host_and_markers() {
        assert!(looks_like_gitlab_input(
            "https://gitlab.example.com/acme/tools/-/blob/main/demo-skill/SKILL.md"
        ));
        assert!(looks_like_gitlab_input("gitlab.example.com/acme/tools"));
        assert!(!looks_like_gitlab_input(
            "https://github.com/anthropics/skills/tree/main/skills/brainstorming"
        ));
        assert!(!looks_like_gitlab_input("https://skills.sh/skills/brainstorming"));
        assert!(!looks_like_gitlab_input("obra/superpowers/brainstorming"));
    }

    #[test]
    fn parse_gitlab_project_only_returns_missing_skill_path() {
        let error = parse_smart_paste_with_hooks(
            "https://gitlab.example.com/acme/tools",
            |_, _, _| Ok(vec![]),
            |_| Ok(MOCK_GITLAB_SKILL_MD.to_string()),
        )
        .expect_err("project-only url should fail");

        assert!(matches!(error, AppError::MissingSkillPath { .. }));
    }
}
