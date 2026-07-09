use crate::models::{default_github_host, AppError, SmartPastePreview};

struct ParsedGitHub {
    owner: String,
    repo: String,
    branch: String,
    directory: String,
}

pub fn parse_smart_paste(input: &str) -> Result<SmartPastePreview, AppError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput {
            input: input.to_string(),
            message: "输入为空".to_string(),
        });
    }

    if looks_like_github_input(trimmed) {
        let parsed = parse_github_input(trimmed)?;
        return Ok(github_to_preview(parsed));
    }

    Err(AppError::InvalidInput {
        input: trimmed.to_string(),
        message: "不支持的链接格式，仅支持 GitHub 公开仓库".to_string(),
    })
}

fn looks_like_github_input(input: &str) -> bool {
    let lower = input.trim().to_lowercase();
    lower.contains("github.com/") || is_owner_repo_path(input)
}

fn is_owner_repo_path(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.contains("://") {
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
        repo_host: default_github_host(),
        project_path: format!("{}/{}", parsed.owner, parsed.repo),
        repo_owner: parsed.owner,
        repo_name: parsed.repo,
        repo_branch: parsed.branch,
        directory: parsed.directory,
        source: "github".to_string(),
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
    fn parse_owner_repo_skill_path_as_github() {
        let preview =
            parse_smart_paste("obra/superpowers/brainstorming").expect("github path parses");

        assert_eq!(preview.repo_host, "github.com");
        assert_eq!(preview.project_path, "obra/superpowers");
        assert_eq!(preview.repo_owner, "obra");
        assert_eq!(preview.repo_name, "superpowers");
        assert_eq!(preview.directory, "brainstorming");
        assert_eq!(preview.source, "github");
    }

    #[test]
    fn rejects_gitlab_url() {
        let error = parse_smart_paste(
            "https://gitlab.example.com/acme/tools/-/blob/main/demo-skill/SKILL.md",
        )
        .expect_err("gitlab url should fail");

        assert!(matches!(error, AppError::InvalidInput { .. }));
        assert!(error.to_dto().message.contains("GitHub"));
    }
}
