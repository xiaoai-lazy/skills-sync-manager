use crate::models::{AppError, DiscoverableSkill, SmartPastePreview};
use reqwest::Url;
use serde::Deserialize;

struct ParsedGitHub {
    owner: String,
    repo: String,
    branch: String,
    directory: String,
}

pub fn parse_smart_paste(input: &str) -> Result<SmartPastePreview, AppError> {
    parse_smart_paste_with_search(input, search_skills_sh)
}

pub fn parse_smart_paste_with_search<F>(
    input: &str,
    search: F,
) -> Result<SmartPastePreview, AppError>
where
    F: FnOnce(&str, u32, u32) -> Result<Vec<DiscoverableSkill>, AppError>,
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

fn is_owner_repo_path(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.contains("://") || trimmed.contains("skills.sh") {
        return false;
    }

    let parts: Vec<&str> = trimmed.split('/').filter(|part| !part.is_empty()).collect();
    parts.len() >= 2 && parts[0] != "tree" && parts[0] != "blob"
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
        repo_owner: parsed.owner,
        repo_name: parsed.repo,
        repo_branch: parsed.branch,
        directory: parsed.directory,
        source: "github".to_string(),
    }
}

fn discoverable_to_preview(skill: &DiscoverableSkill) -> SmartPastePreview {
    SmartPastePreview {
        name: skill.name.clone(),
        description: skill.description.clone(),
        install_dir_name: skill.install_dir_name.clone(),
        repo_owner: skill.repo_owner.clone(),
        repo_name: skill.repo_name.clone(),
        repo_branch: skill.repo_branch.clone(),
        directory: skill.directory.clone(),
        source: skill.source.clone(),
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

    #[test]
    fn parse_tree_url_extracts_owner_repo_branch_and_directory() {
        let preview = parse_smart_paste(
            "https://github.com/anthropics/skills/tree/main/skills/brainstorming",
        )
        .expect("tree url parses");

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
}
